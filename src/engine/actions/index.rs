//! 인덱스 유지보수를 위한 DBEngine 공용 헬퍼 (#217)

use std::collections::HashMap;

use crate::engine::DBEngine;
use crate::engine::ast::types::TableName;
use crate::engine::index::{IndexMeta, field_to_key};
use crate::engine::optimizer::cost::BLOCK_SIZE;
use crate::engine::optimizer::predule::{OptimizerContext, TableStatistics};
use crate::engine::schema::row::{TableDataFieldType, TableDataRow};
use crate::errors;
use crate::errors::execute_error::ExecuteError;

/// 인덱스 이름을 데이터베이스 범위로 한정합니다.
/// IndexManager는 이름을 전역 키로 사용하므로 데이터베이스 간 충돌을 막습니다.
pub(crate) fn qualified_index_name(database_name: &str, index_name: &str) -> String {
    format!("{}.{}", database_name, index_name)
}

/// 행의 특정 컬럼 값을 인덱스 키로 변환합니다.
/// NULL 값은 색인하지 않습니다 (PostgreSQL과 동일하게 unique 제약에서도 제외).
pub(crate) fn row_index_key(row: &TableDataRow, column_name: &str) -> Option<String> {
    row.fields
        .iter()
        .find(|field| field.column_name == column_name)
        .and_then(|field| match &field.data {
            TableDataFieldType::Null => None,
            data => Some(field_to_key(data)),
        })
}

impl DBEngine {
    /// 서버 기동 후 최초 인덱스 사용 시점에 디스크의 인덱스 파일을 메모리로 적재합니다.
    pub(crate) async fn ensure_indices_loaded(&self) -> errors::Result<()> {
        self.indices_loaded
            .get_or_try_init(|| async {
                let base_path = self.get_data_directory();

                if !base_path.exists() {
                    return Ok(());
                }

                let mut entries = tokio::fs::read_dir(&base_path).await.map_err(|error| {
                    ExecuteError::wrap(format!("failed to read data directory: {}", error))
                })?;

                while let Some(entry) = entries.next_entry().await.map_err(|error| {
                    ExecuteError::wrap(format!("failed to read data directory entry: {}", error))
                })? {
                    let path = entry.path();

                    if !path.is_dir() {
                        continue;
                    }

                    if let Some(database_name) = path.file_name().and_then(|name| name.to_str()) {
                        self.index_manager
                            .load_database_indices(database_name)
                            .await?;
                    }
                }

                Ok(())
            })
            .await
            .map(|_| ())
    }

    /// 테이블에 속한 인덱스 메타 목록을 반환합니다.
    pub(crate) async fn table_index_metas(&self, table_name: &TableName) -> Vec<IndexMeta> {
        self.index_manager.indices_for_table(table_name).await
    }

    /// 인덱스 항목 하나를 (old_key -> new_key)로 반영합니다. UPDATE 유지보수용.
    pub(crate) async fn apply_index_operation(
        &self,
        index_name: &str,
        old_key: &Option<String>,
        new_key: &Option<String>,
        row_path: &str,
    ) -> errors::Result<()> {
        match (old_key, new_key) {
            (Some(old_key), Some(new_key)) => {
                self.index_manager
                    .update(index_name, old_key, new_key.clone(), row_path.to_string())
                    .await
            }
            (Some(old_key), None) => self
                .index_manager
                .remove(index_name, old_key, row_path)
                .await
                .map(|_| ()),
            (None, Some(new_key)) => {
                self.index_manager
                    .insert(index_name, new_key.clone(), row_path.to_string())
                    .await
            }
            (None, None) => Ok(()),
        }
    }

    /// 테이블 통계를 반환합니다. 캐시가 없으면 실제 스캔으로 계산 후 캐싱합니다.
    pub(crate) async fn table_statistics(
        &self,
        table_name: &TableName,
    ) -> errors::Result<TableStatistics> {
        if let Some(statistics) = self.statistics_manager.get(table_name).await {
            return Ok(statistics);
        }

        let row_count = self.full_scan(table_name.clone()).await?.len();

        let segment_path = self.row_segment_path(table_name)?;
        let file_size = tokio::fs::metadata(&segment_path)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let block_count = file_size.div_ceil(BLOCK_SIZE).max(1) as usize;

        let mut distinct_values = HashMap::new();
        for meta in self.table_index_metas(table_name).await {
            if let Ok(distinct) = self.index_manager.distinct_keys(&meta.index_name).await {
                distinct_values.insert(meta.column_name.clone(), distinct);
            }
        }

        let statistics = TableStatistics {
            row_count,
            block_count,
            distinct_values,
        };

        self.statistics_manager
            .set(table_name.clone(), statistics.clone())
            .await;

        Ok(statistics)
    }

    /// 옵티마이저 컨텍스트를 구성합니다.
    /// 실패해도 쿼리는 FullScan으로 동작해야 하므로 오류는 빈 컨텍스트로 흡수하지만,
    /// 디버깅을 위해 warn 로그를 남깁니다.
    pub(crate) async fn build_optimizer_context(&self, table_name: &TableName) -> OptimizerContext {
        if let Err(error) = self.ensure_indices_loaded().await {
            log::warn!(
                "build_optimizer_context: ensure_indices_loaded failed for {:?}: {}",
                table_name,
                error
            );
            return OptimizerContext::default();
        }

        let indexes = self.table_index_metas(table_name).await;

        if indexes.is_empty() {
            return OptimizerContext::default();
        }

        let statistics = match self.table_statistics(table_name).await {
            Ok(statistics) => Some(statistics),
            Err(error) => {
                log::warn!(
                    "build_optimizer_context: table_statistics failed for {:?}: {}",
                    table_name,
                    error
                );
                None
            }
        };

        OptimizerContext {
            indexes,
            statistics,
        }
    }
}
