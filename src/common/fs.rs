use futures::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSystemEntry {
    pub path: PathBuf,
    pub is_file: bool,
}

#[mockall::automock]
#[async_trait::async_trait]
pub trait FileSystem {
    async fn create_dir(&self, path: &str) -> io::Result<()>;
    async fn write_file(&self, path: &str, content: &[u8]) -> io::Result<()>;
    async fn read_dir(&self, path: &str) -> io::Result<Vec<FileSystemEntry>>;
    async fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
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

    async fn read_dir(&self, path: &str) -> io::Result<Vec<FileSystemEntry>> {
        let mut directory = tokio::fs::read_dir(path).await?;
        let mut entries = Vec::new();

        while let Some(entry) = directory.next_entry().await? {
            entries.push(FileSystemEntry {
                path: entry.path(),
                is_file: entry.file_type().await?.is_file(),
            });
        }

        Ok(entries)
    }

    async fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        tokio::fs::read(path).await
    }
}
