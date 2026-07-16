#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanType {
    FullScan,
    IndexScan(IndexScanPlan),
}

/// 인덱스 스캔 실행 계획.
/// 키 문자열은 engine::index::field_to_key로 인코딩된 값입니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexScanPlan {
    pub index_name: String,
    pub column_name: String,
    /// 동등(=) 조건 조회 키. 있으면 point lookup으로 실행됩니다.
    pub eq_key: Option<String>,
    /// 범위 시작 키(포함). 배타 경계는 플랜 생성 시점에 "\0" suffix로 보정됩니다.
    pub start_key: Option<String>,
    /// 범위 끝 키(제외)
    pub end_key: Option<String>,
}
