use std::collections::HashMap;

use tokio::sync::RwLock;

use crate::engine::ast::types::TableName;

/// 테이블 단위 통계 정보 (#195)
///
/// 비용 기반 옵티마이저가 스캔 방식을 선택할 때 사용합니다.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableStatistics {
    /// 테이블의 행 개수
    pub row_count: usize,
    /// 행 세그먼트 파일 크기 기반 블록 개수 추정치
    pub block_count: usize,
    /// 컬럼별 고유값 개수 추정치 (인덱스가 있는 컬럼만 채워짐)
    pub distinct_values: HashMap<String, usize>,
}

/// 테이블 통계 캐시 관리자.
///
/// - 최초 접근 시 실제 스캔 결과로 채워짐 (DBEngine::table_statistics)
/// - INSERT/DELETE 시 행 개수를 증분 갱신
/// - DDL 변경 시 invalidate
pub struct StatisticsManager {
    statistics: RwLock<HashMap<TableName, TableStatistics>>,
}

impl StatisticsManager {
    pub fn new() -> Self {
        Self {
            statistics: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get(&self, table: &TableName) -> Option<TableStatistics> {
        self.statistics.read().await.get(table).cloned()
    }

    pub async fn set(&self, table: TableName, statistics: TableStatistics) {
        self.statistics.write().await.insert(table, statistics);
    }

    /// INSERT 반영: 캐시된 통계가 있으면 행 개수와 블록 개수를 증분 갱신합니다.
    pub async fn record_insert(&self, table: &TableName, count: usize) {
        if let Some(statistics) = self.statistics.write().await.get_mut(table) {
            let old_row_count = statistics.row_count;
            statistics.row_count += count;

            // 블록 개수를 비례적으로 갱신 (행 당 평균 블록 수)
            if old_row_count > 0 {
                let avg_rows_per_block = old_row_count / statistics.block_count.max(1);
                if avg_rows_per_block > 0 {
                    statistics.block_count += count.div_ceil(avg_rows_per_block);
                }
            }
        }
    }

    /// DELETE 반영: 캐시된 통계가 있으면 행 개수와 블록 개수를 증분 갱신합니다.
    pub async fn record_delete(&self, table: &TableName, count: usize) {
        if let Some(statistics) = self.statistics.write().await.get_mut(table) {
            let old_row_count = statistics.row_count;
            statistics.row_count = statistics.row_count.saturating_sub(count);

            // 블록 개수를 비례적으로 감소 (소량 삭제는 block_count에 영향 없음)
            if old_row_count > 0 {
                let avg_rows_per_block = old_row_count / statistics.block_count.max(1);
                if avg_rows_per_block > 0 {
                    let removed_blocks = count / avg_rows_per_block;
                    statistics.block_count = statistics.block_count.saturating_sub(removed_blocks);
                }
            }
        }
    }

    pub async fn invalidate(&self, table: &TableName) {
        self.statistics.write().await.remove(table);
    }

    /// 데이터베이스 단위 통계 무효화 (DROP DATABASE 등)
    pub async fn invalidate_database(&self, database_name: &str) {
        self.statistics
            .write()
            .await
            .retain(|table, _| table.database_name.as_deref() != Some(database_name));
    }
}

impl Default for StatisticsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> TableName {
        TableName::new(Some("rrdb".to_string()), "users".to_string())
    }

    #[tokio::test]
    async fn record_insert_and_delete_adjust_row_count() {
        let manager = StatisticsManager::new();

        // 캐시가 없으면 no-op
        manager.record_insert(&table(), 10).await;
        assert_eq!(manager.get(&table()).await, None);

        manager
            .set(
                table(),
                TableStatistics {
                    row_count: 100,
                    block_count: 1,
                    distinct_values: HashMap::new(),
                },
            )
            .await;

        manager.record_insert(&table(), 10).await;
        assert_eq!(manager.get(&table()).await.unwrap().row_count, 110);

        manager.record_delete(&table(), 20).await;
        assert_eq!(manager.get(&table()).await.unwrap().row_count, 90);

        // saturating_sub 확인
        manager.record_delete(&table(), 1000).await;
        assert_eq!(manager.get(&table()).await.unwrap().row_count, 0);
    }

    #[tokio::test]
    async fn invalidate_removes_statistics() {
        let manager = StatisticsManager::new();
        manager.set(table(), TableStatistics::default()).await;

        manager.invalidate(&table()).await;
        assert_eq!(manager.get(&table()).await, None);
    }

    #[tokio::test]
    async fn invalidate_database_removes_only_matching_tables() {
        let manager = StatisticsManager::new();
        let other = TableName::new(Some("other".to_string()), "users".to_string());

        manager.set(table(), TableStatistics::default()).await;
        manager.set(other.clone(), TableStatistics::default()).await;

        manager.invalidate_database("rrdb").await;
        assert_eq!(manager.get(&table()).await, None);
        assert!(manager.get(&other).await.is_some());
    }

    #[tokio::test]
    async fn record_delete_block_count_is_not_over_decremented_for_small_deletes() {
        let manager = StatisticsManager::new();

        // 100 rows, 10 blocks → 10 rows/block
        manager
            .set(
                table(),
                TableStatistics {
                    row_count: 100,
                    block_count: 10,
                    distinct_values: HashMap::new(),
                },
            )
            .await;

        // 3 rows 삭제: 1 block 미만이므로 block_count는 변하지 않아야 함
        manager.record_delete(&table(), 3).await;
        let stats = manager.get(&table()).await.unwrap();
        assert_eq!(stats.row_count, 97);
        assert_eq!(
            stats.block_count, 10,
            "small delete should not reduce block_count"
        );

        // 10 rows 추가 삭제: 정확히 1 block 분량
        manager.record_delete(&table(), 10).await;
        let stats = manager.get(&table()).await.unwrap();
        assert_eq!(stats.row_count, 87);
        assert_eq!(
            stats.block_count, 9,
            "10 rows = 1 block should reduce block_count by 1"
        );
    }
}
