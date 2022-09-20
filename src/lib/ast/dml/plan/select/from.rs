use crate::lib::ast::predule::{Index, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectFromPlan {
    pub table_name: TableName,
    pub alias: Option<String>,
    pub select_columns: Vec<String>,
    pub index: Option<Index>,
}
