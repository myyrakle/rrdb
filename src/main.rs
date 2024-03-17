use std::error::Error;

pub mod ast;
pub mod command;
pub mod constants;
pub mod errors;
pub mod executor;
pub mod lexer;
pub mod logger;
pub mod optimizer;
pub mod parser;
pub mod pgwire;
pub mod server;
pub mod utils;

use command::{Command, SubCommand};
use constants::predule::{DEFAULT_HOST, DEFAULT_PORT};
use executor::predule::Executor;
use server::predule::{Server, ServerOption};

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send>> {
    let args = Command::parse();

    match args.action {
        SubCommand::Init(init) => {
            let init_option = init.init;

            let executor = Executor::new();

            let path = match init_option.config_path {
                Some(path) => path,
                None => ".".into(),
            };

            executor.init(path).await?;
        }
        SubCommand::Run(run) => {
            let server_option = ServerOption {
                port: run.value.port.unwrap_or(DEFAULT_PORT),
                host: run.value.host.unwrap_or_else(|| DEFAULT_HOST.into()),
            };
            let server = Server::new(server_option);

            server.run().await?;
        }
        SubCommand::Client => {
            println!("Client");
            unimplemented!();
        }
    }

    Ok(())
}
