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
    /// 파일을 지정한 크기로 자릅니다. (#268)
    /// WAL 복구 시 newest segment의 torn tail을 제거하는 데 사용합니다.
    async fn truncate(&self, path: &Path, size: u64) -> io::Result<()>;
    /// 파일의 크기(bytes)를 반환합니다. (#265)
    /// `read_segment_rows`가 파일 전체를 메모리로 읽기 전에 예산을 확보하는 데 사용합니다.
    async fn metadata(&self, path: &Path) -> io::Result<u64>;
    /// 디렉토리와 그 내용 전체를 재귀적으로 삭제합니다. (#220)
    /// create_table 실패 시 생성 중인 테이블 디렉토리를 정리하는 데 사용합니다.
    ///
    /// 트레이트 하위 호환을 위해 default 구현을 제공합니다 — 기존 외부
    /// 구현체는 이 메서드 없이도 컴파일이 유지됩니다.
    async fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        tokio::fs::remove_dir_all(path).await
    }
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

    async fn truncate(&self, path: &Path, size: u64) -> io::Result<()> {
        let file = tokio::fs::OpenOptions::new().write(true).open(path).await?;
        file.set_len(size).await
    }

    async fn metadata(&self, path: &Path) -> io::Result<u64> {
        let metadata = tokio::fs::metadata(path).await?;
        Ok(metadata.len())
    }

    async fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        tokio::fs::remove_dir_all(path).await
    }
}
