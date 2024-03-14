use crate::ast::predule::{Index, JoinType, SQLExpression, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct JoinPlan {
    pub left: TableName,
    pub right: TableName,
    pub join_type: JoinType,
    pub join_scan_type: JoinScanType,
    pub select_columns: Vec<String>,
    pub index: Option<Index>,
    pub filter: Option<SQLExpression>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinScanType {
    NestedLoop,
    Hash,
    Merge,
}
