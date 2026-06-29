use serde::Deserialize;

use clap::Args;

/// Config options for running the server.
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptions {
    /// RRDB 파일이 세팅된 기본 디렉터리
    #[clap(name = "base-path", long, short)]
    pub base_path: Option<String>,
}

#[derive(Clone, Debug, Args)]
#[clap(name = "run")]
pub struct Command {
    #[clap(flatten)]
    pub value: ConfigOptions,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::command::{Command, SubCommand};

    #[test]
    fn run_accepts_base_path() {
        let command = Command::parse_from(["rrdb", "run", "--base-path", "local-test"]);

        match command.action {
            SubCommand::Run(run) => {
                assert_eq!(run.value.base_path, Some("local-test".to_string()));
            }
            _ => panic!("expected run command"),
        }
    }

    #[test]
    fn run_rejects_config() {
        let result = Command::try_parse_from(["rrdb", "run", "--config", "local-test/rrdb.config"]);

        assert!(result.is_err());
    }
}
