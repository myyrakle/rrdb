use std::io::ErrorKind as IOErrorKind;

use crate::engine::DBEngine;
use crate::engine::ast::ddl::alter_table::{AlterColumnAction, AlterTableAction, AlterTableQuery};
use crate::engine::ast::types::TableName;
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::table::TableSchema;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors::execute_error::ExecuteError;
use crate::errors::{self, ErrorKind, Errors};

impl DBEngine {
    pub async fn alter_table(&self, query: AlterTableQuery) -> errors::Result<ExecuteResult> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_data_directory();

        let TableName {
            database_name,
            table_name,
        } = query.table.clone().unwrap();

        let database_name = database_name.unwrap();

        let database_path = base_path.clone().join(&database_name);
        let table_path = database_path.clone().join("tables").join(&table_name);

        match query.action {
            AlterTableAction::AlterTableRenameTo(action) => {
                let change_name = action.name;
                let change_path = database_path.clone().join(&change_name);

                // table 디렉터리명 변경
                if let Err(error) = tokio::fs::rename(&table_path, &change_path).await {
                    return Err(ExecuteError::wrap(format!(
                        "table rename failed: {}",
                        error
                    )));
                }

                // config data 파일 내용 변경
                let config_path = change_path.clone().join("table.config");

                let mut table_config = self.get_table_config(query.table.unwrap()).await?;

                table_config.table.table_name = change_name;
                if let Err(error) =
                    tokio::fs::write(config_path, encoder.encode(table_config)).await
                {
                    return Err(ExecuteError::wrap(error.to_string()));
                }
            }
            AlterTableAction::AddColumn(action) => {
                // TODO: 실 데이터 목록에도 반영하기

                // config data 파일 내용 변경
                let config_path = table_path.clone().join("table.config");

                let column_to_add = action.column;

                let mut table_config = self.get_table_config(query.table.unwrap()).await?;

                if table_config.columns.contains(&column_to_add) {
                    return Err(ExecuteError::wrap(format!(
                        "column '{}' already exists ",
                        column_to_add.name
                    )));
                }

                table_config.columns.push(column_to_add);

                if let Err(error) =
                    tokio::fs::write(config_path, encoder.encode(table_config)).await
                {
                    return Err(ExecuteError::wrap(error.to_string()));
                }
            }
            AlterTableAction::AlterColumn(action) => {
                // TODO: 실 데이터 목록에도 반영하기

                let column_name = action.column_name;

                match action.action {
                    AlterColumnAction::AlterColumnSetDefault(action) => {
                        // config data 파일 내용 변경
                        let config_path = table_path.clone().join("table.config");

                        let mut table_config = self.get_table_config(query.table.unwrap()).await?;

                        let target = table_config
                            .columns
                            .iter_mut()
                            .find(|e| e.name == column_name);

                        match target {
                            Some(target) => {
                                target.default = Some(action.expression);
                            }
                            None => {
                                return Err(ExecuteError::wrap(format!(
                                    "column '{}' not exists ",
                                    column_name
                                )));
                            }
                        }

                        if let Err(error) =
                            tokio::fs::write(config_path, encoder.encode(table_config)).await
                        {
                            return Err(ExecuteError::wrap(error.to_string()));
                        }
                    }
                    AlterColumnAction::AlterColumnDropDefault(_) => {
                        // config data 파일 내용 변경
                        let config_path = table_path.clone().join("table.config");

                        match tokio::fs::read(&config_path).await {
                            Ok(data) => {
                                let table_config: Option<TableSchema> =
                                    encoder.decode(data.as_slice());

                                match table_config {
                                    Some(mut table_config) => {
                                        let target = table_config
                                            .columns
                                            .iter_mut()
                                            .find(|e| e.name == column_name);

                                        match target {
                                            Some(target) => {
                                                target.default = None;
                                            }
                                            None => {
                                                return Err(ExecuteError::wrap(format!(
                                                    "column '{}' not exists ",
                                                    column_name
                                                )));
                                            }
                                        }

                                        if let Err(error) = tokio::fs::write(
                                            config_path,
                                            encoder.encode(table_config),
                                        )
                                        .await
                                        {
                                            return Err(ExecuteError::wrap(error.to_string()));
                                        }
                                    }
                                    None => {
                                        return Err(Errors::new(ErrorKind::ExecuteError(
                                            "invalid config data".to_string(),
                                        )));
                                    }
                                }
                            }
                            Err(error) => match error.kind() {
                                IOErrorKind::NotFound => {
                                    return Err(Errors::new(ErrorKind::ExecuteError(
                                        "table not found".to_string(),
                                    )));
                                }
                                _ => {
                                    return Err(ExecuteError::wrap(format!("{:?}", error)));
                                }
                            },
                        }
                    }
                    AlterColumnAction::AlterColumnSetNotNull => {
                        // config data 파일 내용 변경
                        let config_path = table_path.clone().join("table.config");

                        match tokio::fs::read(&config_path).await {
                            Ok(data) => {
                                let table_config: Option<TableSchema> =
                                    encoder.decode(data.as_slice());

                                match table_config {
                                    Some(mut table_config) => {
                                        let target = table_config
                                            .columns
                                            .iter_mut()
                                            .find(|e| e.name == column_name);

                                        match target {
                                            Some(target) => {
                                                target.not_null = true;
                                            }
                                            None => {
                                                return Err(ExecuteError::wrap(format!(
                                                    "column '{}' not exists ",
                                                    column_name
                                                )));
                                            }
                                        }

                                        if let Err(error) = tokio::fs::write(
                                            config_path,
                                            encoder.encode(table_config),
                                        )
                                        .await
                                        {
                                            return Err(ExecuteError::wrap(error.to_string()));
                                        }
                                    }
                                    None => {
                                        return Err(Errors::new(ErrorKind::ExecuteError(
                                            "invalid config data".to_string(),
                                        )));
                                    }
                                }
                            }
                            Err(error) => match error.kind() {
                                IOErrorKind::NotFound => {
                                    return Err(Errors::new(ErrorKind::ExecuteError(
                                        "table not found".to_string(),
                                    )));
                                }
                                _ => {
                                    return Err(ExecuteError::wrap(format!("{:?}", error)));
                                }
                            },
                        }
                    }
                    AlterColumnAction::AlterColumnDropNotNull => {
                        // config data 파일 내용 변경
                        let config_path = table_path.clone().join("table.config");

                        match tokio::fs::read(&config_path).await {
                            Ok(data) => {
                                let table_config: Option<TableSchema> =
                                    encoder.decode(data.as_slice());

                                match table_config {
                                    Some(mut table_config) => {
                                        let target = table_config
                                            .columns
                                            .iter_mut()
                                            .find(|e| e.name == column_name);

                                        match target {
                                            Some(target) => {
                                                target.not_null = false;
                                            }
                                            None => {
                                                return Err(ExecuteError::wrap(format!(
                                                    "column '{}' not exists ",
                                                    column_name
                                                )));
                                            }
                                        }

                                        if let Err(error) = tokio::fs::write(
                                            config_path,
                                            encoder.encode(table_config),
                                        )
                                        .await
                                        {
                                            return Err(ExecuteError::wrap(error.to_string()));
                                        }
                                    }
                                    None => {
                                        return Err(Errors::new(ErrorKind::ExecuteError(
                                            "invalid config data".to_string(),
                                        )));
                                    }
                                }
                            }
                            Err(error) => match error.kind() {
                                IOErrorKind::NotFound => {
                                    return Err(Errors::new(ErrorKind::ExecuteError(
                                        "table not found".to_string(),
                                    )));
                                }
                                _ => {
                                    return Err(ExecuteError::wrap(format!("{:?}", error)));
                                }
                            },
                        }
                    }
                    AlterColumnAction::AlterColumnSetType(action) => {
                        let config_path = table_path.clone().join("table.config");

                        match tokio::fs::read(&config_path).await {
                            Ok(data) => {
                                let table_config: Option<TableSchema> =
                                    encoder.decode(data.as_slice());

                                match table_config {
                                    Some(mut table_config) => {
                                        let target = table_config
                                            .columns
                                            .iter_mut()
                                            .find(|e| e.name == column_name);

                                        match target {
                                            Some(target) => {
                                                target.data_type = action.data_type;
                                            }
                                            None => {
                                                return Err(ExecuteError::wrap(format!(
                                                    "column '{}' not exists ",
                                                    column_name
                                                )));
                                            }
                                        }

                                        if let Err(error) = tokio::fs::write(
                                            config_path,
                                            encoder.encode(table_config),
                                        )
                                        .await
                                        {
                                            return Err(ExecuteError::wrap(error.to_string()));
                                        }
                                    }
                                    None => {
                                        return Err(Errors::new(ErrorKind::ExecuteError(
                                            "invalid config data".to_string(),
                                        )));
                                    }
                                }
                            }
                            Err(error) => match error.kind() {
                                IOErrorKind::NotFound => {
                                    return Err(Errors::new(ErrorKind::ExecuteError(
                                        "table not found".to_string(),
                                    )));
                                }
                                _ => {
                                    return Err(ExecuteError::wrap(format!("{:?}", error)));
                                }
                            },
                        }
                    }
                }
            }
            AlterTableAction::DropColumn(action) => {
                // TODO: 실 데이터 목록에도 반영하기

                // config data 파일 내용 변경
                let config_path = table_path.clone().join("table.config");

                let mut table_config = self.get_table_config(query.table.unwrap()).await?;

                if !table_config
                    .columns
                    .iter()
                    .any(|e| e.name == action.column_name)
                {
                    return Err(ExecuteError::wrap(format!(
                        "column '{}' not exists ",
                        action.column_name
                    )));
                }

                table_config
                    .columns
                    .retain(|e| e.name != action.column_name);

                if let Err(error) =
                    tokio::fs::write(config_path, encoder.encode(table_config)).await
                {
                    return Err(ExecuteError::wrap(error.to_string()));
                }
            }
            AlterTableAction::RenameColumn(action) => {
                // TODO: 실 데이터 목록에도 반영하기

                // config data 파일 내용 변경
                let config_path = table_path.clone().join("table.config");

                let mut table_config = self.get_table_config(query.table.unwrap()).await?;

                if table_config
                    .columns
                    .iter()
                    .any(|e| e.name == action.to_name)
                {
                    return Err(ExecuteError::wrap(format!(
                        "column '{}' already exists ",
                        action.to_name
                    )));
                }

                let target = table_config
                    .columns
                    .iter_mut()
                    .find(|e| e.name == action.from_name);

                match target {
                    Some(target) => {
                        target.name = action.to_name;
                    }
                    None => {
                        return Err(ExecuteError::wrap(format!(
                            "column '{}' not exists ",
                            action.from_name
                        )));
                    }
                }

                if let Err(error) =
                    tokio::fs::write(config_path, encoder.encode(table_config)).await
                {
                    return Err(ExecuteError::wrap(error.to_string()));
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
