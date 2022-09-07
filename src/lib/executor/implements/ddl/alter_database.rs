use std::error::Error;
use std::io::ErrorKind;

use crate::lib::ast::ddl::AlterDatabaseQuery;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::encoder::StorageEncoder;
use crate::lib::executor::predule::{DatabaseConfig, ExecuteResult, Executor};
use crate::lib::executor::result::{ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteRow};

impl Executor {
    pub async fn alter_database(
        &self,
        query: AlterDatabaseQuery,
    ) -> Result<ExecuteResult, Box<dyn Error>> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_base_path();
        let mut database_path = base_path.clone();

        let database_name = query
            .database_name
            .clone()
            .ok_or_else(|| ExecuteError::boxed("no database name"))?;

        database_path.push(&database_name);

        #[allow(clippy::single_match)]
        match tokio::fs::create_dir(database_path.clone()).await {
            Ok(()) => {
                // 성공
            }
            Err(error) => match error.kind() {
                ErrorKind::AlreadyExists => {
                    return Err(ExecuteError::boxed("already exists database"))
                }
                _ => {
                    return Err(ExecuteError::boxed("database create failed"));
                }
            },
        }

        // 각 데이터베이스 단위 설정파일 생성
        database_path.push("database.config");
        let database_info = DatabaseConfig {
            database_name: database_name.clone(),
        };

        tokio::fs::write(database_path, encoder.encode(database_info)).await?;

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "database created: {}",
                    database_name
                ))],
            }]),
        })
    }
}
