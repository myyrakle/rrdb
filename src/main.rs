pub mod command;
pub mod lib;

use command::commands::SubCommand;
use lib::constants::server::DEFAULT_PORT;
use lib::server::{Server, ServerOption};

use clap::Parser;
use std::fs::create_dir;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    action: SubCommand,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.action {
        SubCommand::Init(_init) => {
            //let configPath = init.init.configPath.unwrap_or("./".into());
            //println!("Init, {:?}", configPath);

            create_dir(".rrdb.config").unwrap();
        }
        SubCommand::Run(run) => {
            let server_option = ServerOption {
                port: run.value.port.unwrap_or(DEFAULT_PORT),
            };
            let server = Server::new(server_option);

            server.run().await;
        }
        SubCommand::Client => {
            println!("Client");
        }
    }

    Ok(())
}
