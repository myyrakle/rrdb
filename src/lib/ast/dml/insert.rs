use crate::lib::ast::predule::TableName;

#[derive(Clone, Debug, PartialEq)]
pub struct InsertQuery {
    pub into_table: Option<TableName>,
}
