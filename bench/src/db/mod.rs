use crate::{DEADLINE, Result};
use sqlx::{
    ConnectOptions, PgPool, Row,
    postgres::{PgConnectOptions, PgPoolOptions},
};

#[async_trait::async_trait]
pub(crate) trait Adapter: Send + Sync {
    async fn execute(&self, sql: &str) -> Result<()>;
    async fn count(&self, sql: &str) -> Result<usize>;
}

// Shared PostgreSQL wire adapter, irrespective of the selected backend label.
pub(crate) struct Pg {
    pool: PgPool,
}

impl Pg {
    pub(crate) async fn connect(url: &str, workers: usize) -> Result<Self> {
        let options = url
            .parse::<PgConnectOptions>()
            .map_err(|_| "invalid BENCH_DATABASE_URL")?
            .disable_statement_logging();
        let pool = PgPoolOptions::new()
            .max_connections(workers as u32)
            .acquire_timeout(DEADLINE)
            .test_before_acquire(false)
            .connect_with(options)
            .await
            .map_err(|_| "database connection failed")?;
        // Establish the exact same number of connections before measuring either backend.
        let mut leases = Vec::with_capacity(workers);
        for _ in 0..workers {
            leases.push(
                pool.acquire()
                    .await
                    .map_err(|_| "database connection failed")?,
            );
        }
        drop(leases);
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl Adapter for Pg {
    async fn execute(&self, sql: &str) -> Result<()> {
        sqlx::raw_sql(sql)
            .execute(&self.pool)
            .await
            .map_err(|_| "database statement failed")?;
        Ok(())
    }

    async fn count(&self, sql: &str) -> Result<usize> {
        let row = sqlx::raw_sql(sql)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| "database readback failed")?;
        let count = row
            .try_get::<i64, _>(0)
            .map_err(|_| "invalid readback count")?;
        usize::try_from(count).map_err(|_| "invalid readback count")
    }
}
