use crate::lib::ast::predule::{Index, JoinType, SQLExpression, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectJoinPlan {
    left: TableName,
    right: TableName,
    join_type: JoinType,
    join_scan_type: JoinScanType,
    select_columns: Vec<String>,
    index: Option<Index>,
    filter: Option<SQLExpression>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinScanType {
    NestedLoop,
    Hash,
    Merge,
}
