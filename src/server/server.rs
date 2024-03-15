use std::error::Error;

use crate::errors::execute_error::ExecuteError;
use crate::executor::predule::Executor;
use crate::logger::predule::Logger;
use crate::pgwire::predule::Connection;
use crate::server::channel::ChannelResponse;
use crate::server::predule::{ChannelRequest, ServerOption, SharedState};

use futures::future::join_all;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use super::client::ClientInfo;

pub struct Server {
    pub option: ServerOption,
}

impl Server {
    pub fn new(option: ServerOption) -> Self {
        Self { option }
    }

    /// 메인 서버 루프.
    /// 여러개의 태스크 제어
    pub async fn run(&self) -> Result<(), Box<dyn Error + Send>> {
        // TODO: 인덱스 로딩 등 기본 로직 실행.

        let (request_sender, mut request_receiver) = mpsc::channel::<ChannelRequest>(1000);

        // background task
        // 쿼리 실행 요청을 전달받음
        let background_task = tokio::spawn(async move {
            while let Some(request) = request_receiver.recv().await {
                tokio::spawn(async move {
                    let executor = Executor::new();
                    let result = executor.process_query(request.statement).await;

                    match result {
                        Ok(result) => {
                            if let Err(_response) = request
                                .response_sender
                                .send(ChannelResponse { result: Ok(result) })
                            {
                                Logger::error("channel send failed");
                            }
                        }
                        Err(error) => {
                            let error = error.to_string();
                            if let Err(_response) = request.response_sender.send(ChannelResponse {
                                result: Err(ExecuteError::boxed(ExecuteError::boxed(error))),
                            }) {
                                Logger::error("channel send failed");
                            }
                        }
                    }
                });
            }
        });

        // connection task
        // client와의 커넥션 처리 루프
        let listener = TcpListener::bind((self.option.host.to_owned(), self.option.port as u16))
            .await
            .map_err(|error| ExecuteError::dyn_boxed(error.to_string()))?;

        let connection_task = tokio::spawn(async move {
            loop {
                let accepted = listener.accept().await;

                let (stream, address) = match accepted {
                    Ok((stream, address)) => (stream, address),
                    Err(error) => {
                        Logger::error(format!("socket error {:?}", error));
                        continue;
                    }
                };

                let client_info = ClientInfo {
                    ip: address.ip(),
                    connection_id: uuid::Uuid::new_v4().to_string(),
                    database: "None".into(),
                };

                let shared_state = SharedState {
                    sender: request_sender.clone(),
                    client_info,
                };

                tokio::spawn(async move {
                    let mut conn = Connection::new(shared_state);
                    if let Err(error) = conn.run(stream).await {
                        Logger::error(format!("connection error {:?}", error));
                    }
                });
            }
        });

        Logger::info(format!(
            "Server is running on {}:{}",
            self.option.host, self.option.port
        ));

        join_all(vec![connection_task, background_task]).await;

        Ok(())
    }
}
