use std::error::Error;
use std::path::PathBuf;

use crate::lib::ast::ddl::CreateDatabaseQuery;
use crate::lib::errors::ExecuteError;
use crate::lib::executor::Executor;
use crate::lib::utils::get_system_env;

impl Executor {
    pub async fn create_database(&self, query: CreateDatabaseQuery) -> Result<(), Box<dyn Error>> {
        let base_path = PathBuf::from(get_system_env("RRDB_BASE_PATH"));
        let mut database_path = base_path.clone();

        let database_name = query
            .database_name
            .clone()
            .ok_or(ExecuteError::boxed("no database name"))?;

        database_path.push(database_name);

        tokio::fs::create_dir(database_path).await?;

        Ok(())
    }
}
