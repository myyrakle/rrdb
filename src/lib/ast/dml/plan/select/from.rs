use crate::lib::ast::predule::{SelectScanType, TableName};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectFromPlan {
    pub table_name: TableName,
    pub alias: Option<String>,
    pub scan: SelectScanType,
}
