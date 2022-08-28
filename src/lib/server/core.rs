use std::sync::Arc;

use crate::lib::ast::predule::{DDLStatement, SQLStatement};
use crate::lib::executor::predule::Executor;
use crate::lib::optimizer::predule::Optimizer;
use crate::lib::parser::predule::Parser;
use crate::lib::pgwire::predule::{Connection, RRDBEngine};
use crate::lib::server::predule::ServerOption;

use tokio::net::TcpListener;

pub struct Server {
    pub option: ServerOption,
}

async fn _process_query(query: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(query)?;
    let executor = Executor::new();

    let mut ast_list = parser.parse()?;

    // 최적화 작업
    let optimizer = Optimizer::new();
    ast_list.iter_mut().for_each(|e| optimizer.optimize(e));

    // 쿼리 실행
    for ast in ast_list {
        match ast {
            SQLStatement::DDL(DDLStatement::CreateDatabaseQuery(query)) => {
                return executor.create_database(query).await;
            }
            _ => {
                println!("?: {:?}", ast);
            }
        }
    }

    Ok(())
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
    pub async fn run(&self) -> std::io::Result<u16> {
        let listener =
            TcpListener::bind((self.option.host.to_owned(), self.option.port as u16)).await?;
        let port = listener.local_addr()?.port();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let engine_func = Arc::new(|| Box::pin(async { RRDBEngine }));
                tokio::spawn(async move {
                    let mut conn = Connection::new(engine_func().await);
                    conn.run(stream).await.unwrap();
                });
            }
        });

        Ok(port)
    }
}
