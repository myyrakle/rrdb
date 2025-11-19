pub mod command;
pub mod common;
pub mod config;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod pgwire;
pub mod utils;

use command::{Command, SubCommand};
use errors::RRDBError;

use clap::Parser;

use crate::{
    config::launch_config::LaunchConfig,
    engine::{DBEngine, server::Server},
};

#[tokio::main]
async fn main() -> Result<(), RRDBError> {
    env_logger::init();

    let args = Command::parse();

    match args.action {
        SubCommand::Init(init) => {
            let config = LaunchConfig::load_from_path(None).unwrap_or_default();

            let _init_option = init.init;

            let engine = DBEngine::new(config);

            engine.initialize().await?;
        }
        SubCommand::Run(run) => {
            let config = LaunchConfig::load_from_path(run.value.config).expect("config load error");

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
