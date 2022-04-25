use crate::lib::ast::TableName;

#[derive(Debug, Clone)]
pub struct InsertQuery {
    pub into_table: Option<TableName>,
}
