use clap::Args;

#[derive(Clone, Debug, Args)]
#[clap(name = "daemon")]
pub struct Command {}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::command::{Command, SubCommand};

    #[test]
    fn parses_daemon_command() {
        let command = Command::parse_from(["rrdb", "daemon"]);

        match command.action {
            SubCommand::Daemon(_) => {}
            _ => panic!("expected daemon command"),
        }
    }
}
