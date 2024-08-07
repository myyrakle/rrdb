use std::process::{Command, Output};

use futures::io;

#[mockall::automock]
#[async_trait::async_trait]
pub trait FileSystem {
    async fn create_dir(&self, path: &str) -> io::Result<()>;
    async fn write_file(&self, path: &str, content: &[u8]) -> io::Result<()>;
}

pub struct RealFileSystem;

#[async_trait::async_trait]
impl FileSystem for RealFileSystem {
    async fn create_dir(&self, path: &str) -> io::Result<()> {
        tokio::fs::create_dir(path).await
    }

    async fn write_file(&self, path: &str, content: &[u8]) -> io::Result<()> {
        tokio::fs::write(path, content).await
    }
}

#[mockall::automock]
pub trait CommandRunner {
    fn run(&self, command: &mut Command) -> io::Result<Output>;
}

pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, command: &mut Command) -> io::Result<Output> {
        command.output()
    }
}
