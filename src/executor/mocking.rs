use futures::io;

#[async_trait::async_trait]
pub trait FileSystem {
    async fn create_dir(&self, path: &str) -> io::Result<()>;
}

pub struct RealFileSystem;

#[async_trait::async_trait]
impl FileSystem for RealFileSystem {
    async fn create_dir(&self, path: &str) -> io::Result<()> {
        tokio::fs::create_dir(path).await
    }
}
