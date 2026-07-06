pub mod client;
pub mod shared_state;

use std::sync::Arc;
use std::time::Duration;

use crate::config::launch_config::LaunchConfig;
use crate::engine::server::client::ClientInfo;
use crate::engine::server::shared_state::SharedState;
use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
use crate::engine::wal::manager::builder::WALBuilder;
use crate::engine::{DBEngine, SharedWALManager};
use crate::errors;
use crate::errors::execute_error::ExecuteError;
use crate::pgwire::connection::ConnectionError;
use crate::pgwire::predule::Connection;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

const DEFAULT_DURABILITY_FLUSH_INTERVAL: Duration = Duration::from_secs(10);

pub struct Server {
    pub config: Arc<LaunchConfig>,
}

fn is_expected_disconnect(error: &ConnectionError) -> bool {
    matches!(error, ConnectionError::ConnectionClosed)
}

fn spawn_durability_flush_loop(
    engine: Arc<DBEngine>,
    wal_manager: SharedWALManager,
    interval_duration: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(interval_duration);

        loop {
            interval.tick().await;

            if let Err(error) = engine.flush_row_buffers_durable().await {
                log::error!("failed to flush row buffers durably: {}", error);
                continue;
            }

            if let Err(error) = wal_manager.lock().await.flush().await {
                log::error!("failed to flush WAL: {}", error);
            }
        }
    })
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
        {
            let mut wal_manager_guard = wal_manager.lock().await;
            engine
                .recover_from_wal(&mut wal_manager_guard)
                .await
                .map_err(|error| ExecuteError::wrap(error.to_string()))?;
        }
        let _durability_flush_task = spawn_durability_flush_loop(
            engine.clone(),
            wal_manager.clone(),
            DEFAULT_DURABILITY_FLUSH_INTERVAL,
        );

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
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use tokio::sync::Mutex;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::DBEngine;
    use crate::engine::ast::types::TableName;
    use crate::engine::schema::row::{TableDataField, TableDataFieldType, TableDataRow};
    use crate::engine::wal::endec::WALDecoder;
    use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
    use crate::engine::wal::manager::builder::WALBuilder;
    use crate::engine::wal::types::EntryType;
    use crate::pgwire::connection::ConnectionError;

    use super::is_expected_disconnect;
    use super::spawn_durability_flush_loop;

    #[test]
    fn connection_closed_is_expected_disconnect() {
        assert!(is_expected_disconnect(&ConnectionError::ConnectionClosed));
    }

    async fn setup_test_wal_dir(test_name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let wal_dir = PathBuf::from("target")
            .join("test_server_wal_data")
            .join(format!("{test_name}_{now}"));
        tokio::fs::create_dir_all(&wal_dir).await.unwrap();
        wal_dir
    }

    #[tokio::test]
    async fn durability_flush_loop_syncs_rows_before_checkpointing_wal() {
        let wal_dir = setup_test_wal_dir("durability_flush_loop").await;
        let base_path = wal_dir.join("base");
        let config = LaunchConfig::default_for_base_path(&base_path);
        let table_name = TableName::new(Some("rrdb".to_string()), "users".to_string());
        let rows_path = PathBuf::from(&config.data_directory)
            .join("rrdb")
            .join("tables")
            .join("users")
            .join("rows");
        tokio::fs::create_dir_all(&rows_path).await.unwrap();
        tokio::fs::create_dir_all(&config.wal_directory)
            .await
            .unwrap();

        let decoder = BincodeDecoder::new();
        let encoder = BincodeEncoder::new();
        let wal_manager = Arc::new(Mutex::new(
            WALBuilder::new(&config)
                .build(decoder.clone(), encoder)
                .await
                .unwrap(),
        ));
        let engine = Arc::new(DBEngine::new(config.clone()));
        let row = TableDataRow {
            fields: vec![TableDataField {
                table_name: table_name.clone(),
                column_name: "id".to_string(),
                data: TableDataFieldType::Integer(1),
            }],
        };

        wal_manager
            .lock()
            .await
            .append_record(EntryType::Insert, Some(b"row-1".to_vec()), None)
            .await
            .unwrap();
        engine.append_table_rows(&table_name, &[row]).await.unwrap();
        engine.flush_row_buffers().await.unwrap();
        assert!(!engine.row_buffer_pool.lock().await.is_unsynced_empty());

        let flush_task =
            spawn_durability_flush_loop(engine.clone(), wal_manager, Duration::from_millis(10));
        let wal_path =
            PathBuf::from(&config.wal_directory).join(format!("00000001.{}", config.wal_extension));
        let segment_path = rows_path.join("00000001.rows");

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let content = tokio::fs::read(&wal_path).await.unwrap();
                let entries = decoder.decode(&content).unwrap();

                if entries
                    .iter()
                    .any(|entry| matches!(entry.entry_type, EntryType::Checkpoint))
                {
                    assert!(segment_path.exists());
                    assert!(engine.row_buffer_pool.lock().await.is_unsynced_empty());
                    break;
                }

                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();

        flush_task.abort();
    }
}
