use std::io::ErrorKind as IOErrorKind;

use crate::engine::DBEngine;
use crate::engine::ast::ddl::drop_table::DropTableQuery;
use crate::engine::ast::types::TableName;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn drop_table(&self, query: DropTableQuery) -> errors::Result<ExecuteResult> {
        let base_path = self.get_data_directory();

        let table = query.table.unwrap();

        self.invalidate_table_config_cache(&table).await;

        let TableName {
            database_name,
            table_name,
        } = table;

        let table_path = base_path
            .clone()
            .join(database_name.unwrap())
            .join("tables")
            .join(&table_name);

        if let Err(error) = tokio::fs::remove_dir_all(table_path).await {
            match error.kind() {
                IOErrorKind::NotFound => {
                    if !query.if_exists {
                        return Err(ExecuteError::wrap("table not found".to_string()));
                    }
                }
                _ => {
                    return Err(ExecuteError::wrap("table drop failed".to_string()));
                }
            }
        }

        Ok(ExecuteResult::new(
            vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }],
            vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "table dropped: {}",
                    table_name
                ))],
            }],
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::DBEngine;
    use crate::engine::ast::ddl::drop_table::DropTableQuery;
    use crate::engine::ast::types::TableName;

    #[tokio::test]
    async fn drop_table_if_exists_succeeds_when_table_is_missing() {
        let base_path = PathBuf::from("target/test_drop_table/if_exists_missing");
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let config = LaunchConfig::default_for_base_path(&base_path);
        tokio::fs::create_dir_all(
            PathBuf::from(&config.data_directory)
                .join("rrdb")
                .join("tables"),
        )
        .await
        .unwrap();

        let engine = DBEngine::new(config);
        engine
            .drop_table(
                DropTableQuery::builder()
                    .set_table(TableName::new(
                        Some("rrdb".to_string()),
                        "missing".to_string(),
                    ))
                    .set_if_exists(true),
            )
            .await
            .unwrap();
    }
}
