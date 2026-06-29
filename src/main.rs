pub mod command;
pub mod common;
pub mod config;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod pgwire;
pub mod utils;

use std::path::PathBuf;

use command::{Command, SubCommand};

use clap::Parser;

use crate::{
    config::launch_config::LaunchConfig,
    engine::{DBEngine, server::Server},
};

#[tokio::main]
async fn main() -> errors::Result<()> {
    env_logger::init();

    let args = Command::parse();

    match args.action {
        SubCommand::Init(init) => {
            let base_path = init.init.base_path.map(PathBuf::from);
            let config_path = base_path
                .as_ref()
                .map(|path| path.join(crate::constants::DEFAULT_CONFIG_FILENAME));
            let config = match config_path.as_ref() {
                Some(path) => {
                    let base_path = base_path.as_ref().unwrap();
                    LaunchConfig::load_from_path(Some(path.to_string_lossy().to_string()))
                        .await
                        .unwrap_or_default()
                        .with_base_path(base_path)
                }
                None => LaunchConfig::load_from_path(None).await.unwrap_or_default(),
            };

            let engine = DBEngine::new(config);

            engine.initialize_with_base_path(base_path).await?;
        }
        SubCommand::Run(run) => {
            let config = LaunchConfig::load_from_path(run.value.config)
                .await
                .expect("config load error");

            let server = Server::new(config);

            server.run().await?;
        }
        SubCommand::Daemon(_) => {
            let config = LaunchConfig::load_from_path(None).await.unwrap_or_default();
            let engine = DBEngine::new(config);

            engine.install_daemon().await?;
        }
        SubCommand::Client => {
            println!("Client");
            unimplemented!();
        }
    }

    Ok(())
}
