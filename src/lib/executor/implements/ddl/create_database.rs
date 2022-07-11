use std::error::Error;
use std::path::PathBuf;

use crate::lib::ast::ddl::CreateDatabaseQuery;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::predule::DatabaseConfig;
use crate::lib::executor::predule::Executor;
use crate::lib::utils::predule::get_system_env;

impl Executor {
    pub async fn create_database(&self, query: CreateDatabaseQuery) -> Result<(), Box<dyn Error>> {
        let base_path = PathBuf::from(get_system_env("RRDB_BASE_PATH"));
        let mut database_path = base_path.clone();

        let database_name = query
            .database_name
            .clone()
            .ok_or(ExecuteError::boxed("no database name"))?;

        database_path.push(&database_name);
        tokio::fs::create_dir(database_path.clone()).await?;

        // 각 데이터베이스 단위 설정파일 생성
        database_path.push("database.config");
        let database_info = DatabaseConfig { database_name };
        let database_config = toml::to_string(&database_info).unwrap();
        tokio::fs::write(database_path, database_config.as_bytes()).await?;

        Ok(())
    }
}
