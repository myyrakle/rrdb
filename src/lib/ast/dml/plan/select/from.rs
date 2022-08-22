use crate::lib::ast::predule::{Index, SQLExpression, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectFromPlan {
    table_name: TableName,
    select_columns: Vec<String>,
    index: Option<Index>,
    filter: Option<SQLExpression>,
}
