use std::error::Error;

use crate::lib::errors::execute_error::ExecuteError;
use crate::lib::executor::executor::Executor;
use crate::lib::executor::result::ExecuteResult;
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

    /// Starts a server using a function responsible for producing engine instances and set of bind options.
    ///
    /// Returns once the server is listening for connections, with the accept loop
    /// running as a background task, and returns the listener's local port.
    ///
    /// Useful for creating test harnesses binding to port 0 to select a random port.
    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        // TODO: 인덱스 로딩 등 기본 로직 실행.

        let (request_sender, mut request_receiver) = mpsc::channel::<ChannelRequest>(1000);

        // background task
        let background_task = tokio::spawn(async move {
            while let Some(request) = request_receiver.recv().await {
                println!("???");
                tokio::spawn(async move {
                    let executor = Executor::new();
                    let result = executor.process_query(request.statement).await;

                    match result {
                        Ok(result) => {
                            request
                                .response_sender
                                .send(ChannelResponse { result: Ok(result) })
                                .unwrap();
                        }
                        Err(error) => {
                            request
                                .response_sender
                                .send(ChannelResponse {
                                    result: Err(ExecuteError::boxed(error.to_string())),
                                })
                                .unwrap();
                        }
                    }
                })
                .await
                .unwrap();
            }
        });

        // connection task
        let listener =
            TcpListener::bind((self.option.host.to_owned(), self.option.port as u16)).await?;

        let connection_task = tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();

                let shared_state = SharedState {
                    sender: request_sender.clone(),
                };

                tokio::spawn(async move {
                    let mut conn = Connection::new(shared_state);
                    conn.run(stream).await.unwrap();
                })
                .await
                .unwrap();
            }
        });

        let result = join!(background_task, connection_task);
        println!("{:?}", result);

        Ok(())
    }
}
