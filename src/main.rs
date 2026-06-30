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
    constants::DEFAULT_CONFIG_FILENAME,
    engine::{DBEngine, server::Server},
    errors::execute_error::ExecuteError,
};

async fn load_launch_config(base_path: Option<&PathBuf>) -> errors::Result<LaunchConfig> {
    match base_path {
        Some(base_path) => {
            let config_path = base_path.join(DEFAULT_CONFIG_FILENAME);

            let config =
                LaunchConfig::load_from_path(Some(config_path.to_string_lossy().to_string()))
                    .await
                    .map_err(|error| ExecuteError::wrap(format!("config load error: {}", error)))?;

            Ok(config.with_base_path(base_path))
        }
        None => LaunchConfig::load_from_path(None)
            .await
            .map_err(|error| ExecuteError::wrap(format!("config load error: {}", error))),
    }
}

fn banner() -> String {
    format!(
        r#"
 ____  ____  ____  ____
|  _ \|  _ \|  _ \| __ )
| |_) | |_) | | | |  _ \
|  _ <|  _ <| |_| | |_) |
|_| \_\_| \_\____/|____/

RRDB v{}
"#,
        env!("CARGO_PKG_VERSION")
    )
}

fn print_banner() {
    println!("{}", banner());
}

#[tokio::main]
async fn main() -> errors::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Command::parse();

    match args.action {
        SubCommand::Init(init) => {
            let base_path = init.init.base_path.map(PathBuf::from);
            let config = match base_path.as_ref() {
                Some(base_path) => load_launch_config(Some(base_path))
                    .await
                    .unwrap_or_else(|_| LaunchConfig::default_for_base_path(base_path)),
                None => load_launch_config(None).await.unwrap_or_default(),
            };

            let engine = DBEngine::new(config);

            engine.initialize_with_base_path(base_path).await?;
        }
        SubCommand::Run(run) => {
            let base_path = run.value.base_path.map(PathBuf::from);
            let config = load_launch_config(base_path.as_ref()).await?;

            print_banner();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_includes_product_name_and_version() {
        let banner = banner();

        assert!(banner.contains("RRDB"));
        assert!(banner.contains(env!("CARGO_PKG_VERSION")));
    }

    #[tokio::test]
    async fn load_launch_config_applies_base_path_to_loaded_config() {
        let base_path = PathBuf::from("target/test_config_loader/run_base_path");
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }
        tokio::fs::create_dir_all(&base_path).await.unwrap();

        let config_path = base_path.join(DEFAULT_CONFIG_FILENAME);
        tokio::fs::write(
            &config_path,
            br#"port = 34567
host = "127.0.0.2"
data_directory = "/var/lib/rrdb/data"
wal_enabled = true
wal_directory = "/var/lib/rrdb/wal"
wal_segment_size = 16777216
wal_extension = "log"
"#,
        )
        .await
        .unwrap();

        let config = load_launch_config(Some(&base_path)).await.unwrap();

        assert_eq!(config.port, 34567);
        assert_eq!(config.host, "127.0.0.2");
        assert_eq!(
            config.data_directory,
            base_path
                .join(constants::DEFAULT_DATA_DIRNAME)
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(
            config.wal_directory,
            base_path
                .join(constants::DEFAULT_WAL_DIRNAME)
                .to_string_lossy()
                .to_string()
        );
    }

    #[tokio::test]
    async fn load_launch_config_returns_error_when_base_path_config_is_missing() {
        let base_path = PathBuf::from("target/test_config_loader/missing_base_path");
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let result = load_launch_config(Some(&base_path)).await;

        assert!(result.is_err());
    }
}
