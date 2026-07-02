use std::io::ErrorKind as IOErrorKind;

use crate::engine::DBEngine;

use crate::engine::ast::ddl::drop_database::DropDatabaseQuery;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn drop_database(&self, query: DropDatabaseQuery) -> errors::Result<ExecuteResult> {
        let base_path = self.get_data_directory();
        let mut database_path = base_path.clone();

        let database_name = query
            .database_name
            .clone()
            .ok_or_else(|| ExecuteError::wrap("no database name".to_string()))?;

        database_path.push(&database_name);

        if let Err(error) = tokio::fs::remove_dir_all(database_path.clone()).await {
            match error.kind() {
                IOErrorKind::NotFound => {
                    if !query.if_exists {
                        return Err(ExecuteError::wrap("database not found".to_string()));
                    }
                }
                _ => {
                    return Err(ExecuteError::wrap("database drop failed".to_string()));
                }
            }
        }

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "database dropped: {}",
                    database_name
                ))],
            }]),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::DBEngine;
    use crate::engine::ast::ddl::drop_database::DropDatabaseQuery;

    #[tokio::test]
    async fn drop_database_if_exists_succeeds_when_database_is_missing() {
        let base_path = PathBuf::from("target/test_drop_database/if_exists_missing");
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let config = LaunchConfig::default_for_base_path(&base_path);
        tokio::fs::create_dir_all(&config.data_directory)
            .await
            .unwrap();

        let engine = DBEngine::new(config);
        engine
            .drop_database(
                DropDatabaseQuery::builder()
                    .set_name("missing".to_string())
                    .set_if_exists(true),
            )
            .await
            .unwrap();
    }
}
