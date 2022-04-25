use crate::lib::ast::Table;

#[derive(Debug, Clone)]
pub struct InsertQuery {
    pub into_table: Option<Table>,
}
