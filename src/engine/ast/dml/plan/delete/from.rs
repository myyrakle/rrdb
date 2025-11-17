use crate::engine::ast::{dml::plan::select::scan::ScanType, types::TableName};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteFromPlan {
    pub table_name: TableName,
    pub alias: Option<String>,
    pub scan: ScanType,
}
