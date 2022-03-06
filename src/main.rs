pub mod command;
pub mod lib;

use command::commands::SubCommands;

use clap::Parser;

use std::fs::create_dir;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    action: SubCommands,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.action {
        SubCommands::Init(_init) => {
            //let configPath = init.init.configPath.unwrap_or("./".into());
            //println!("Init, {:?}", configPath);

            create_dir(".rrdb.config").unwrap();
        }
        SubCommands::Run => {
            println!("Run");

            // loop {

            //     break;
            // }
        }
        SubCommands::Client => {
            println!("Client");
        }
    }

    Ok(())
}
