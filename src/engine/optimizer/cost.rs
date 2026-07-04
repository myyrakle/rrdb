//! 비용 기반 옵티마이저의 비용 상수 및 공식 (#195)
//!
//! 상수 값은 PostgreSQL의 기본 비용 파라미터를 참조했습니다.

/// 순차 페이지 읽기 비용
pub const SEQ_PAGE_COST: f64 = 1.0;
/// 임의 페이지 읽기 비용
pub const RANDOM_PAGE_COST: f64 = 4.0;
/// 튜플 하나 처리 비용
pub const CPU_TUPLE_COST: f64 = 0.01;
/// 연산자 하나 평가 비용
pub const CPU_OPERATOR_COST: f64 = 0.0025;
/// 고유값 통계가 없을 때 사용하는 동등 조건 선택도 기본값
pub const DEFAULT_EQ_SELECTIVITY: f64 = 0.005;
/// 범위 조건 선택도 기본값
pub const DEFAULT_RANGE_SELECTIVITY: f64 = 1.0 / 3.0;
/// 블록 개수 추정에 사용하는 블록 크기 (bytes)
pub const BLOCK_SIZE: u64 = 8192;

/// 풀 스캔 비용: 모든 블록 순차 읽기 + 모든 튜플 처리
pub fn full_scan_cost(row_count: usize, block_count: usize) -> f64 {
    block_count.max(1) as f64 * SEQ_PAGE_COST + row_count as f64 * CPU_TUPLE_COST
}

/// 인덱스 스캔 비용: B-tree 탐색 + 매칭 튜플별 임의 접근
pub fn index_scan_cost(row_count: usize, selectivity: f64) -> f64 {
    let rows = row_count as f64;
    let matched = (rows * selectivity).max(1.0);
    let btree_descent = if rows > 1.0 { rows.log2().ceil() } else { 1.0 } * CPU_OPERATOR_COST;

    btree_descent + matched * (RANDOM_PAGE_COST + CPU_TUPLE_COST + CPU_OPERATOR_COST)
}

/// 동등 조건 선택도: 고유값 개수를 알면 1/ndv, 모르면 기본값
pub fn eq_selectivity(distinct_values: Option<usize>) -> f64 {
    match distinct_values {
        Some(count) if count > 0 => (1.0 / count as f64).min(1.0),
        _ => DEFAULT_EQ_SELECTIVITY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_scan_beats_full_scan_for_selective_predicate_on_large_table() {
        let row_count = 10_000;
        let block_count = 100;

        let full = full_scan_cost(row_count, block_count);
        let index = index_scan_cost(row_count, eq_selectivity(Some(row_count)));

        assert!(index < full);
    }

    #[test]
    fn full_scan_beats_index_scan_on_tiny_table() {
        let row_count = 3;
        let block_count = 1;

        let full = full_scan_cost(row_count, block_count);
        let index = index_scan_cost(row_count, eq_selectivity(Some(row_count)));

        assert!(full < index);
    }

    #[test]
    fn full_scan_beats_unselective_range_scan() {
        let row_count = 10_000;
        let block_count = 100;

        let full = full_scan_cost(row_count, block_count);
        let index = index_scan_cost(row_count, DEFAULT_RANGE_SELECTIVITY);

        // 행의 1/3을 임의 접근으로 읽는 것은 풀 스캔보다 비쌈
        assert!(full < index);
    }

    #[test]
    fn eq_selectivity_uses_distinct_values() {
        assert_eq!(eq_selectivity(Some(100)), 0.01);
        assert_eq!(eq_selectivity(Some(0)), DEFAULT_EQ_SELECTIVITY);
        assert_eq!(eq_selectivity(None), DEFAULT_EQ_SELECTIVITY);
    }
}
