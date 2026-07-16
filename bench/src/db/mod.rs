use std::{fmt::Debug, sync::Arc};

pub mod rrdb;

#[async_trait::async_trait]
pub trait Database {
    // connection ping
    async fn ping(&self) -> Result<()>;

    // create table if not exists
    // re-create table if exists
    async fn setup(&self) -> Result<()>;

    // write key, value
    async fn write(&self, key: &str, value: &str) -> Result<()>;

    fn worker_count(&self) -> usize {
        10000
    }
}

pub async fn new_database(_db_type: &str) -> Result<Arc<dyn Database + Send + Sync>> {
    rrdb::Rrdb::new().await
}

pub enum Errors {
    ConnectionError(String),
    WriteError(String),
    ReadError,
}

impl Debug for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Errors::ConnectionError(msg) => write!(f, "ConnectionError: {}", msg),
            Errors::WriteError(msg) => write!(f, "WriteError: {}", msg),
            Errors::ReadError => write!(f, "ReadError"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Errors>;

#[derive(Clone, Debug)]
pub struct FakeDB {}

impl FakeDB {
    pub fn new() -> Arc<dyn Database + Send + Sync> {
        Arc::new(FakeDB {})
    }
}

#[async_trait::async_trait]
impl Database for FakeDB {
    async fn ping(&self) -> Result<()> {
        Ok(())
    }

    async fn setup(&self) -> Result<()> {
        Ok(())
    }

    async fn write(&self, _key: &str, _value: &str) -> Result<()> {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        Ok(())
    }
}
