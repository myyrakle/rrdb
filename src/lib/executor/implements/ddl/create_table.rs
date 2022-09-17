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
    ) -> Result<ExecuteResult, Box<dyn Error>> {
        let encoder = StorageEncoder::new();

        let database_name = query.table.clone().unwrap().database_name.unwrap();
        let table_name = query.table.clone().unwrap().table_name;

        let base_path = self.get_base_path();
        let mut database_path = base_path.clone();

        database_path.push(&database_name);

        let mut table_path = database_path.clone();
        table_path.push(&table_name);

        #[allow(clippy::single_match)]
        match tokio::fs::create_dir(&table_path).await {
            Ok(()) => {
                // 성공
            }
            Err(error) => match error.kind() {
                ErrorKind::AlreadyExists => {
                    return Err(ExecuteError::boxed("already exists table"))
                }
                _ => {
                    return Err(ExecuteError::boxed("table create failed"));
                }
            },
        }

        // 각 데이터베이스 단위 설정파일 생성
        table_path.push("table.config");
        let table_info: TableConfig = query.into();

        tokio::fs::write(table_path, encoder.encode(table_info)).await?;

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
                    database_name
                ))],
            }]),
        })
    }
}
