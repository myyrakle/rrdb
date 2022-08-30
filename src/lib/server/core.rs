use std::error::Error;
use std::sync::Arc;

use crate::lib::ast::predule::{DDLStatement, SQLStatement};
use crate::lib::errors::server_error::ServerError;
use crate::lib::executor::predule::Executor;
use crate::lib::optimizer::predule::Optimizer;
use crate::lib::parser::predule::Parser;
use crate::lib::pgwire::predule::{Connection, RRDBEngine};
use crate::lib::server::predule::{ChannelRequest, ChannelResponse, ServerOption};

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
        let (response_sender, mut response_receiver) = mpsc::channel::<ChannelResponse>(1000);

        // background task
        tokio::spawn(async move {
            while let Some(request) = request_receiver.recv().await {
                //

                response_sender.send(ChannelResponse {}).await;
            }
        });

        // connection task
        let listener =
            TcpListener::bind((self.option.host.to_owned(), self.option.port as u16)).await?;

        let result = tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let engine_func = Arc::new(|| {
                    Box::pin(async {
                        RRDBEngine {
                            request_sender,
                            response_receiver,
                        }
                    })
                });
                tokio::spawn(async move {
                    let mut conn = Connection::new(engine_func().await);
                    conn.run(stream).await.unwrap();
                });
            }
        })
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(error) => Err(ServerError::boxed(error.to_string())),
        }
    }
}
