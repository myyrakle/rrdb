use std::io::ErrorKind as IOErrorKind;

use crate::engine::DBEngine;
use crate::engine::ast::ddl::create_table::CreateTableQuery;
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::table::TableSchema;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors::execute_error::ExecuteError;
use crate::errors::{self, ErrorKind, Errors};

impl DBEngine {
    pub async fn create_table(&self, query: CreateTableQuery) -> errors::Result<ExecuteResult> {
        let encoder = StorageEncoder::new();

        let database_name = query.table.clone().unwrap().database_name.unwrap();
        let table_name = query.table.clone().unwrap().table_name;

        let base_path = self.get_data_directory();
        let database_path = base_path.clone().join(&database_name);

        let table_path = database_path.clone().join("tables").join(&table_name);

        if let Err(error) = tokio::fs::create_dir(&table_path).await {
            match error.kind() {
                IOErrorKind::AlreadyExists => {
                    return Err(Errors::new(ErrorKind::ExecuteError(
                        "already exists table".to_string(),
                    )));
                }
                _ => {
                    return Err(Errors::new(ErrorKind::ExecuteError(
                        "table create failed".to_string(),
                    )));
                }
            }
        }

        // 각 데이터베이스 단위 설정파일 생성
        let config_path = table_path.clone().join("table.config");
        let table_info: TableSchema = query.into();

        if let Err(error) = tokio::fs::write(&config_path, encoder.encode(table_info)).await {
            return Err(ExecuteError::wrap(error.to_string()));
        }

        let rows_path = table_path.clone().join("rows");

        // 데이터 경로 생성
        if let Err(error) = tokio::fs::create_dir(&rows_path).await {
            return Err(ExecuteError::wrap(error.to_string()));
        }

        let index_path = table_path.clone().join("index");

        // 인덱스 경로 생성
        if let Err(error) = tokio::fs::create_dir(&index_path).await {
            return Err(ExecuteError::wrap(error.to_string()));
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
