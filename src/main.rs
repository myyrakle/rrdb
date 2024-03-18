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

            executor.init().await?;
        }
        SubCommand::Run(run) => {
            let server_option = ServerOption {
                port: run.value.port,
                host: run.value.host,
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
