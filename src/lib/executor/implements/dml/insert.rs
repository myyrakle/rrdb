use std::error::Error;
use std::io::ErrorKind;

use crate::lib::ast::predule::InsertQuery;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::predule::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow, Executor,
    StorageEncoder, TableConfig,
};

impl Executor {
    pub async fn insert(&self, query: InsertQuery) -> Result<ExecuteResult, Box<dyn Error>> {
        let encoder = StorageEncoder::new();

        let into_table = query.into_table.unwrap();

        let database_name = into_table.database_name.clone().unwrap();
        let table_name = into_table.table_name;

        let base_path = self.get_base_path();

        let database_path = base_path.clone().join(&database_name);

        let table_path = database_path.clone().join(&table_name);

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
        let config_path = table_path.join("table.config");

        let table_config = match tokio::fs::read(&config_path).await {
            Ok(data) => {
                let table_config: Option<TableConfig> = encoder.decode(data.as_slice());

                match table_config {
                    Some(table_config) => table_config,
                    None => {
                        return Err(ExecuteError::boxed("invalid config data"));
                    }
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::NotFound => {
                    return Err(ExecuteError::boxed("table not found"));
                }
                _ => {
                    return Err(ExecuteError::boxed(format!("{:?}", error)));
                }
            },
        };

        // 필수 입력 컬럼값 검증
        let grequired_columns_map = table_config.get_required_columns_map();

        // 입력값 타입 검증

        // 입력값의 내부 표현식 계산

        let columns_map = table_config.get_columns_map();

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "inserted into {}",
                    table_name
                ))],
            }]),
        })
    }
}
