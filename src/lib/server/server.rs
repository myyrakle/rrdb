use std::error::Error;

use crate::lib::errors::execute_error::ExecuteError;
use crate::lib::executor::predule::Executor;
use crate::lib::logger::predule::Logger;
use crate::lib::pgwire::predule::Connection;
use crate::lib::server::channel::ChannelResponse;
use crate::lib::server::predule::{ChannelRequest, ServerOption, SharedState};

use tokio::join;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

pub struct Server {
    pub option: ServerOption,
}

impl Server {
    pub fn new(option: ServerOption) -> Self {
        Self { option }
    }

    /// 메인 서버 루프.
    /// 여러개의 태스크 제어
    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        // TODO: 인덱스 로딩 등 기본 로직 실행.

        let (request_sender, mut request_receiver) = mpsc::channel::<ChannelRequest>(1000);

        // background task
        // 쿼리 실행 요청을 전달받음
        tokio::spawn(async move {
            while let Some(request) = request_receiver.recv().await {
                tokio::spawn(async move {
                    let executor = Executor::new();
                    let result = executor.process_query(request.statement).await;

                    match result {
                        Ok(result) => {
                            if let Err(_channel_response) = request
                                .response_sender
                                .send(ChannelResponse { result: Ok(result) })
                            {
                                Logger::error("channel send failed");
                            }
                        }
                        Err(error) => {
                            if let Err(_channel_response) =
                                request.response_sender.send(ChannelResponse {
                                    result: Err(ExecuteError::boxed(error.to_string())),
                                })
                            {
                                Logger::error("channel send failed");
                            }
                        }
                    }
                });
            }
        });

        // connection task
        // client와의 커넥션 처리 루프
        let listener =
            TcpListener::bind((self.option.host.to_owned(), self.option.port as u16)).await?;

        tokio::spawn(async move {
            loop {
                let accepted = listener.accept().await;

                let stream = match accepted {
                    Ok((stream, _)) => stream,
                    Err(error) => {
                        Logger::error(format!("socket error {:?}", error));
                        continue;
                    }
                };

                let shared_state = SharedState {
                    sender: request_sender.clone(),
                    database: "None".into(),
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

        Ok(())
    }
}
