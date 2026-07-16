use crate::engine::DBEngine;
use crate::engine::SharedWALManager;
use crate::engine::actions::index::qualified_index_name;
use crate::engine::ast::ddl::drop_index::DropIndexQuery;
use crate::engine::types::ExecuteResult;
use crate::engine::wal::types::EntryType;
use crate::errors;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn drop_index(
        &self,
        query: DropIndexQuery,
        wal_manager: SharedWALManager,
    ) -> errors::Result<ExecuteResult> {
        self.ensure_indices_loaded().await?;

        let database_name = Self::drop_index_database_name(&query)?;
        let index_name = qualified_index_name(&database_name, &query.index_name);

        if self.index_manager.get_meta(&index_name).await.is_none() {
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

        // WAL-first: 인덱스 매니저를 변경하기 전에 먼저 durable하게 기록합니다.
        let wal_payload =
            bincode::serialize(&query).map_err(|error| ExecuteError::wrap(error.to_string()))?;
        wal_manager
            .lock()
            .await
            .append_record(EntryType::DropIndex, Some(wal_payload), None)
            .await?;

        self.drop_index_apply(&query).await
    }

    /// Re-applies a previously WAL-logged DROP INDEX during crash recovery
    /// replay. Idempotent: if the index is already gone, treats it as
    /// already-applied instead of failing.
    pub(crate) async fn drop_index_replay(
        &self,
        query: DropIndexQuery,
    ) -> errors::Result<ExecuteResult> {
        self.ensure_indices_loaded().await?;
        self.drop_index_apply(&query).await
    }

    async fn drop_index_apply(&self, query: &DropIndexQuery) -> errors::Result<ExecuteResult> {
        let database_name = Self::drop_index_database_name(query)?;
        let index_name = qualified_index_name(&database_name, &query.index_name);

        let meta = match self.index_manager.get_meta(&index_name).await {
            Some(meta) => meta,
            None => {
                return Ok(Self::index_result(format!(
                    "index not found, skipped: {}",
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

    fn drop_index_database_name(query: &DropIndexQuery) -> errors::Result<String> {
        query
            .database_name
            .clone()
            .or_else(|| {
                query
                    .table
                    .as_ref()
                    .and_then(|table| table.database_name.clone())
            })
            .ok_or_else(|| ExecuteError::wrap("database name is required".to_string()))
    }
}
