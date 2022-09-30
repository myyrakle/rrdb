use std::error::Error;
use std::io::ErrorKind;

use crate::lib::ast::ddl::CreateTableQuery;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::encoder::StorageEncoder;
use crate::lib::executor::predule::{ExecuteResult, Executor, TableConfig};
use crate::lib::executor::result::{ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteRow};

impl Executor {
    pub async fn create_table(
        &self,
        query: CreateTableQuery,
    ) -> Result<ExecuteResult, Box<dyn Error + Send>> {
        let encoder = StorageEncoder::new();

        let database_name = query.table.clone().unwrap().database_name.unwrap();
        let table_name = query.table.clone().unwrap().table_name;

        let base_path = self.get_base_path();
        let database_path = base_path.clone().join(&database_name);

        let table_path = database_path.clone().join("tables").join(&table_name);

        if let Err(error) = tokio::fs::create_dir(&table_path).await {
            match error.kind() {
                ErrorKind::AlreadyExists => {
                    return Err(ExecuteError::boxed("already exists table"))
                }
                _ => {
                    return Err(ExecuteError::boxed("table create failed"));
                }
            }
        }

        // 각 데이터베이스 단위 설정파일 생성
        let config_path = table_path.clone().join("table.config");
        let table_info: TableConfig = query.into();

        if let Err(error) = tokio::fs::write(&config_path, encoder.encode(table_info)).await {
            return Err(ExecuteError::boxed(error.to_string()));
        }

        let rows_path = table_path.clone().join("rows");

        // 데이터 경로 생성
        if let Err(error) = tokio::fs::create_dir(&rows_path).await {
            return Err(ExecuteError::boxed(error.to_string()));
        }

        let index_path = table_path.clone().join("index");

        // 인덱스 경로 생성
        if let Err(error) = tokio::fs::create_dir(&index_path).await {
            return Err(ExecuteError::boxed(error.to_string()));
        }

        // TODO: primary key 데이터 생성
        // TODO: unique key 데이터 생성
        // TODO: foreign key 데이터 생성

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "table created: {}",
                    table_name
                ))],
            }]),
        })
    }
}
