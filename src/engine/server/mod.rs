pub mod client;
pub mod shared_state;

use std::sync::Arc;

use crate::config::launch_config::LaunchConfig;
use crate::engine::DBEngine;
use crate::engine::server::client::ClientInfo;
use crate::engine::server::shared_state::SharedState;
use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
use crate::engine::wal::manager::builder::WALBuilder;
use crate::errors;
use crate::errors::execute_error::ExecuteError;
use crate::pgwire::connection::ConnectionError;
use crate::pgwire::predule::Connection;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

pub struct Server {
    pub config: Arc<LaunchConfig>,
}

fn is_expected_disconnect(error: &ConnectionError) -> bool {
    matches!(error, ConnectionError::ConnectionClosed)
}

impl Server {
    pub fn new(config: LaunchConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// 메인 서버 루프.
    /// 여러개의 태스크 제어
    pub async fn run(&self) -> errors::Result<()> {
        // TODO: 인덱스 로딩 등 기본 로직 실행.

        let engine = Arc::new(DBEngine::new(self.config.as_ref().clone()));

        // WAL 관리자 생성
        let encoder = BincodeEncoder::new();
        let decoder = BincodeDecoder::new();

        let wal_manager = Arc::new(Mutex::new(
            WALBuilder::new(&self.config)
                .build(decoder, encoder)
                .await
                .map_err(|error| ExecuteError::wrap(error.to_string()))?,
        ));

        // Background WAL flush loop: periodically syncs the current WAL file
        // to disk. Individual WAL writes only call write_all() (no flush),
        // so this loop is the primary durability boundary. The interval
        // trades off between crash-loss window and write throughput.
        //
        // The lock is only held long enough to snapshot the current file path;
        // the sync_data (fsync) call happens outside the lock to avoid
        // blocking all WAL operations during disk I/O.
        let bg_wal_manager = wal_manager.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(200));
            loop {
                interval.tick().await;
                let path = {
                    let mgr = bg_wal_manager.lock().await;
                    mgr.current_file_path()
                };
                // Sync outside the lock so WAL writes aren't blocked during fsync.
                match tokio::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .await
                {
                    Ok(file) => {
                        if let Err(error) = file.sync_data().await {
                            log::warn!("background WAL flush failed: {}", error);
                        }
                    }
                    Err(error) => {
                        log::warn!("background WAL flush failed: {}", error);
                    }
                }
            }
        });

        // connection task
        // client와의 커넥션 처리 루프
        let listener = TcpListener::bind((self.config.host.to_owned(), self.config.port as u16))
            .await
            .map_err(|error| ExecuteError::wrap(error.to_string()))?;

        let connection_task = tokio::spawn(async move {
            loop {
                let accepted = listener.accept().await;

                let (stream, address) = match accepted {
                    Ok((stream, address)) => (stream, address),
                    Err(error) => {
                        log::error!("socket error {:?}", error);
                        continue;
                    }
                };

                let client_info = ClientInfo {
                    ip: address.ip(),
                    connection_id: uuid::Uuid::new_v4().to_string(),
                    database: "None".into(),
                };

                let shared_state = SharedState {
                    engine: engine.clone(),
                    wal_manager: wal_manager.clone(),
                    client_info,
                };

                tokio::spawn(async move {
                    let mut conn = Connection::new(shared_state);
                    if let Err(error) = conn.run(stream).await {
                        if is_expected_disconnect(&error) {
                            log::debug!("connection closed");
                        } else {
                            log::error!("connection error {:?}", error);
                        }
                    }
                });
            }
        });

        log::info!(
            "Server is running on {}:{}",
            self.config.host,
            self.config.port
        );

        connection_task
            .await
            .map_err(|error| ExecuteError::wrap(error.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::pgwire::connection::ConnectionError;

    use super::is_expected_disconnect;

    #[test]
    fn connection_closed_is_expected_disconnect() {
        assert!(is_expected_disconnect(&ConnectionError::ConnectionClosed));
    }
}
