use crate::engine::DBEngine;
use crate::engine::actions::index::qualified_index_name;
use crate::engine::ast::ddl::drop_index::DropIndexQuery;
use crate::engine::types::ExecuteResult;
use crate::errors;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn drop_index(&self, query: DropIndexQuery) -> errors::Result<ExecuteResult> {
        self.ensure_indices_loaded().await?;

        let database_name = query
            .database_name
            .clone()
            .or_else(|| {
                query
                    .table
                    .as_ref()
                    .and_then(|table| table.database_name.clone())
            })
            .ok_or_else(|| ExecuteError::wrap("database name is required".to_string()))?;

        let index_name = qualified_index_name(&database_name, &query.index_name);

        let meta = match self.index_manager.get_meta(&index_name).await {
            Some(meta) => meta,
            None => {
                if query.if_exists {
                    return Ok(Self::index_result(format!(
                        "index not found, skipped: {}",
                        query.index_name
                    )));
                }

                return Err(ExecuteError::wrap(format!(
                    "index '{}' not found",
                    query.index_name
                )));
            }
        };

        self.index_manager.drop_index(&index_name).await?;

        // distinct 통계에서 해당 인덱스가 빠지도록 캐시 무효화
        self.statistics_manager.invalidate(&meta.table_name).await;

        Ok(Self::index_result(format!(
            "index dropped: {}",
            query.index_name
        )))
    }
}
