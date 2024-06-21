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

use std::sync::Arc;

use command::{Command, SubCommand};
use errors::RRDBError;
use executor::{config::global::GlobalConfig, predule::Executor};
use server::predule::Server;

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), RRDBError> {
    let args = Command::parse();

    match args.action {
        SubCommand::Init(init) => {
            let _init_option = init.init;

            let executor = Executor::new(Arc::new(GlobalConfig::default()));

            executor.init().await?;
        }
        SubCommand::Run(run) => {
            let config = GlobalConfig::load_from_path(run.value.config);

            let server = Server::new(config);

            server.run().await?;
        }
        SubCommand::Client => {
            println!("Client");
            unimplemented!();
        }
    }

    Ok(())
}
