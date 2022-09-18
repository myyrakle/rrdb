use std::error::Error;
use std::io::ErrorKind;

use crate::lib::ast::ddl::AlterColumnAction;
use crate::lib::ast::predule::{AlterTableAction, AlterTableQuery, TableName};
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::predule::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow, Executor,
    StorageEncoder, TableConfig,
};

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
            AlterTableAction::AddColumn(action) => {
                // TODO: 실 데이터 목록에도 반영하기

                // config data 파일 내용 변경
                let config_path = table_path.clone().join("table.config");

                let column_to_add = action.column;

                match tokio::fs::read(&config_path).await {
                    Ok(data) => {
                        let table_config: Option<TableConfig> = encoder.decode(data.as_slice());

                        match table_config {
                            Some(mut table_config) => {
                                if table_config.columns.contains(&column_to_add) {
                                    return Err(ExecuteError::boxed(format!(
                                        "column '{}' already exists ",
                                        column_to_add.name
                                    )));
                                }

                                table_config.columns.push(column_to_add);

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
            AlterTableAction::AlterColumn(action) => {
                // TODO: 실 데이터 목록에도 반영하기

                let column_name = action.column_name;

                match action.action {
                    AlterColumnAction::AlterColumnSetDefault(action) => {
                        // config data 파일 내용 변경
                        let config_path = table_path.clone().join("table.config");

                        match tokio::fs::read(&config_path).await {
                            Ok(data) => {
                                let table_config: Option<TableConfig> =
                                    encoder.decode(data.as_slice());

                                match table_config {
                                    Some(mut table_config) => {
                                        let target = table_config
                                            .columns
                                            .iter_mut()
                                            .find(|e| &e.name == &column_name);

                                        match target {
                                            Some(target) => {
                                                target.default = Some(action.expression);
                                            }
                                            None => {
                                                return Err(ExecuteError::boxed(format!(
                                                    "column '{}' not exists ",
                                                    column_name
                                                )));
                                            }
                                        }

                                        tokio::fs::write(config_path, encoder.encode(table_config))
                                            .await?;
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
                    AlterColumnAction::AlterColumnDropDefault(_) => {
                        // config data 파일 내용 변경
                        let config_path = table_path.clone().join("table.config");

                        match tokio::fs::read(&config_path).await {
                            Ok(data) => {
                                let table_config: Option<TableConfig> =
                                    encoder.decode(data.as_slice());

                                match table_config {
                                    Some(mut table_config) => {
                                        let target = table_config
                                            .columns
                                            .iter_mut()
                                            .find(|e| &e.name == &column_name);

                                        match target {
                                            Some(target) => {
                                                target.default = None;
                                            }
                                            None => {
                                                return Err(ExecuteError::boxed(format!(
                                                    "column '{}' not exists ",
                                                    column_name
                                                )));
                                            }
                                        }

                                        tokio::fs::write(config_path, encoder.encode(table_config))
                                            .await?;
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
                    AlterColumnAction::AlterColumnSetNotNull => {
                        // config data 파일 내용 변경
                        let config_path = table_path.clone().join("table.config");

                        match tokio::fs::read(&config_path).await {
                            Ok(data) => {
                                let table_config: Option<TableConfig> =
                                    encoder.decode(data.as_slice());

                                match table_config {
                                    Some(mut table_config) => {
                                        let target = table_config
                                            .columns
                                            .iter_mut()
                                            .find(|e| &e.name == &column_name);

                                        match target {
                                            Some(target) => {
                                                target.not_null = true;
                                            }
                                            None => {
                                                return Err(ExecuteError::boxed(format!(
                                                    "column '{}' not exists ",
                                                    column_name
                                                )));
                                            }
                                        }

                                        tokio::fs::write(config_path, encoder.encode(table_config))
                                            .await?;
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
                    AlterColumnAction::AlterColumnDropNotNull => {
                        // config data 파일 내용 변경
                        let config_path = table_path.clone().join("table.config");

                        match tokio::fs::read(&config_path).await {
                            Ok(data) => {
                                let table_config: Option<TableConfig> =
                                    encoder.decode(data.as_slice());

                                match table_config {
                                    Some(mut table_config) => {
                                        let target = table_config
                                            .columns
                                            .iter_mut()
                                            .find(|e| &e.name == &column_name);

                                        match target {
                                            Some(target) => {
                                                target.not_null = false;
                                            }
                                            None => {
                                                return Err(ExecuteError::boxed(format!(
                                                    "column '{}' not exists ",
                                                    column_name
                                                )));
                                            }
                                        }

                                        tokio::fs::write(config_path, encoder.encode(table_config))
                                            .await?;
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
                    AlterColumnAction::AlterColumnSetType(action) => {}
                }
            }
            AlterTableAction::DropColumn(action) => {
                // TODO: 실 데이터 목록에도 반영하기

                // config data 파일 내용 변경
                let config_path = table_path.clone().join("table.config");

                match tokio::fs::read(&config_path).await {
                    Ok(data) => {
                        let table_config: Option<TableConfig> = encoder.decode(data.as_slice());

                        match table_config {
                            Some(mut table_config) => {
                                if let None = table_config
                                    .columns
                                    .iter()
                                    .find(|e| &e.name == &action.column_name)
                                {
                                    return Err(ExecuteError::boxed(format!(
                                        "column '{}' not exists ",
                                        action.column_name
                                    )));
                                }

                                table_config
                                    .columns
                                    .retain(|e| e.name != action.column_name);

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
            AlterTableAction::RenameColumn(action) => {
                // TODO: 실 데이터 목록에도 반영하기

                // config data 파일 내용 변경
                let config_path = table_path.clone().join("table.config");

                match tokio::fs::read(&config_path).await {
                    Ok(data) => {
                        let table_config: Option<TableConfig> = encoder.decode(data.as_slice());

                        match table_config {
                            Some(mut table_config) => {
                                if table_config
                                    .columns
                                    .iter()
                                    .any(|e| &e.name == &action.to_name)
                                {
                                    return Err(ExecuteError::boxed(format!(
                                        "column '{}' already exists ",
                                        action.to_name
                                    )));
                                }

                                let target = table_config
                                    .columns
                                    .iter_mut()
                                    .find(|e| &e.name == &action.from_name);

                                match target {
                                    Some(target) => {
                                        target.name = action.to_name;
                                    }
                                    None => {
                                        return Err(ExecuteError::boxed(format!(
                                            "column '{}' not exists ",
                                            action.from_name
                                        )));
                                    }
                                }

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
