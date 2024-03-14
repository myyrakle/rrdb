use crate::ast::{dml::plan::select::scan::ScanType, predule::TableName};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateFromPlan {
    pub table_name: TableName,
    pub alias: Option<String>,
    pub scan: ScanType,
}
