use crate::command::init::InitCommand;
use crate::command::run::RunCommand;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Command {
    #[clap(subcommand)]
    pub action: SubCommand,
}

#[derive(clap::Subcommand, Debug)]
pub enum SubCommand {
    /// 설정 파일 및 저장공간 초기화
    Init(InitCommand),
    /// DB 서버 실행
    Run(RunCommand),
    /// DB 클라이언트 실행
    Client,
}
