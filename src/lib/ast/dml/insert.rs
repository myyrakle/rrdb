use crate::lib::ast::predule::{DMLStatement, SQLStatement, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct InsertQuery {
    pub into_table: Option<TableName>,
}

impl From<InsertQuery> for SQLStatement {
    fn from(value: InsertQuery) -> SQLStatement {
        SQLStatement::DML(DMLStatement::InsertQuery(value))
    }
}
