use std::sync::Arc;

use sqlx::{PgPool, postgres::PgPoolOptions};

use super::{Database, Errors, Result};

#[derive(Debug)]
pub struct Rrdb {
    pool: PgPool,
}

impl Rrdb {
    pub async fn new() -> Result<Arc<dyn Database + Send + Sync>> {
        let connection_string = "postgres://rrdb@127.0.0.1:22208/rrdb";

        let pool = PgPoolOptions::new()
            .max_connections(1000)
            .min_connections(1000)
            .connect(connection_string)
            .await
            .map_err(|error| Errors::ConnectionError(error.to_string()))?;

        Ok(Arc::new(Rrdb { pool }))
    }
}

#[async_trait::async_trait]
impl Database for Rrdb {
    async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|error| Errors::ConnectionError(error.to_string()))?;

        Ok(())
    }

    async fn setup(&self) -> Result<()> {
        sqlx::query("DROP TABLE IF EXISTS key_value")
            .execute(&self.pool)
            .await
            .map_err(|error| Errors::WriteError(error.to_string()))?;

        sqlx::query(
            "CREATE TABLE key_value (
                key VARCHAR(255) PRIMARY KEY,
                value VARCHAR(65535)
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|error| Errors::WriteError(error.to_string()))?;

        Ok(())
    }

    async fn write(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT INTO key_value (key, value) VALUES ($1, $2)")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|error| Errors::WriteError(error.to_string()))?;

        Ok(())
    }

    fn worker_count(&self) -> usize {
        1000
    }
}
