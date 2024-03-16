use crate::ast::types::TableName;

use super::scan::ScanType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectFromPlan {
    pub table_name: TableName,
    pub alias: Option<String>,
    pub scan: ScanType,
}
