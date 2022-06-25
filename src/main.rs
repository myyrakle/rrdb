pub mod command;
pub mod lib;

use command::commands::{Command, SubCommand};
use lib::constants::server::{DEFAULT_HOST, DEFAULT_PORT};
use lib::executor::Executor;
use lib::server::{Server, ServerOption};

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
                host: run.value.host.unwrap_or(DEFAULT_HOST.into()),
            };
            let server = Server::new(server_option);

            server.run().await?;
        }
        SubCommand::Client => {
            println!("Client");
        }
    }

    Ok(())
}
