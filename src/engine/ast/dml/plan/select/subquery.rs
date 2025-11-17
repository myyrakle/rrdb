use crate::engine::ast::types::{Index, SQLExpression, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectSubqueryPlan {
    pub table_name: TableName,
    pub select_columns: Vec<String>,
    pub index: Option<Index>,
    pub filter: Option<SQLExpression>,
}
