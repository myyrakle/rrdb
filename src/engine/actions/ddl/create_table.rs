use std::io::ErrorKind as IOErrorKind;

use crate::engine::DBEngine;
use crate::engine::actions::index::qualified_index_name;
use crate::engine::ast::ddl::create_table::CreateTableQuery;
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::index::IndexMeta;
use crate::engine::schema::table::TableSchema;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors;
use crate::errors::execute_error::ExecuteError;

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
                    return Err(ExecuteError::wrap("already exists table".to_string()));
                }
                _ => {
                    return Err(ExecuteError::wrap("table create failed".to_string()));
                }
            }
        }

        // 각 데이터베이스 단위 설정파일 생성
        let config_path = table_path.clone().join("table.config");
        let table_info: TableSchema = query.into();

        if let Err(error) = tokio::fs::write(&config_path, encoder.encode(table_info.clone())).await
        {
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

        // PRIMARY KEY 자동 인덱스 생성 (#217)
        let primary_key_columns: Vec<String> = if table_info.primary_key.is_empty() {
            table_info
                .columns
                .iter()
                .filter(|column| column.primary_key)
                .map(|column| column.name.clone())
                .collect()
        } else {
            table_info.primary_key.clone()
        };

        // TODO(#217): 복합 PRIMARY KEY 인덱스는 미지원 (단일 컬럼만 자동 생성)
        if primary_key_columns.len() == 1 {
            self.ensure_indices_loaded().await?;

            let index_name = qualified_index_name(&database_name, &format!("{}_pkey", table_name));
            let meta = IndexMeta::new(
                index_name,
                table_info.table.clone(),
                primary_key_columns[0].clone(),
                true,
            );

            self.index_manager.create_index(meta).await?;
        }

        // TODO: unique key 데이터 생성
        // TODO: foreign key 데이터 생성

        self.cache_table_config(table_info).await;

        Ok(ExecuteResult::new(
            vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }],
            vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "table created: {}",
                    table_name
                ))],
            }],
        ))
    }
}
