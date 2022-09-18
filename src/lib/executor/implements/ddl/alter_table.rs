use std::error::Error;
use std::io::ErrorKind;

use crate::lib::ast::ddl::{AlterTableAction, AlterTableQuery};
use crate::lib::ast::predule::TableName;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::encoder::StorageEncoder;
use crate::lib::executor::predule::{ExecuteResult, Executor, TableConfig};
use crate::lib::executor::result::{ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteRow};

impl Executor {
    pub async fn alter_table(
        &self,
        query: AlterTableQuery,
    ) -> Result<ExecuteResult, Box<dyn Error>> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_base_path();

        let TableName {
            database_name,
            table_name,
        } = query.table.unwrap();

        let database_name = database_name.unwrap();

        let database_path = base_path.clone().join(&database_name);
        let table_path = database_path.clone().join(&table_name);

        match query.action {
            AlterTableAction::AlterTableRenameTo(action) => {
                let change_name = action.name;
                let change_path = database_path.clone().join(&change_name);

                // table 디렉터리명 변경
                if let Err(error) = tokio::fs::rename(&table_path, &change_path).await {
                    return Err(ExecuteError::boxed(format!(
                        "table rename failed: {}",
                        error.to_string()
                    )));
                }

                // config data 파일 내용 변경
                let config_path = change_path.clone().join("table.config");

                match tokio::fs::read(&config_path).await {
                    Ok(data) => {
                        let table_config: Option<TableConfig> = encoder.decode(data.as_slice());

                        match table_config {
                            Some(mut table_config) => {
                                table_config.table.table_name = change_name;
                                tokio::fs::write(config_path, encoder.encode(table_config)).await?;
                            }
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
                }
            }
            AlterTableAction::AddColumn(action) => {}
            AlterTableAction::AlterColumn(action) => {}
            AlterTableAction::DropColumn(action) => {}
            AlterTableAction::RenameColumn(action) => {}
            AlterTableAction::None => {}
        }

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String("alter table".into())],
            }]),
        })
    }
}
