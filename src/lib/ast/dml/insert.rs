use crate::lib::ast::predule::{DMLStatement, SQLStatement, TableName};

use super::InsertValue;

#[derive(Clone, Debug, PartialEq)]
pub struct InsertQuery {
    pub into_table: Option<TableName>,
    pub columns: Vec<String>,
    pub values: Vec<InsertValue>,
}

impl InsertQuery {
    pub fn builder() -> Self {
        Self {
            columns: vec![],
            into_table: None,
            values: vec![],
        }
    }

    pub fn set_into_table(mut self, from: TableName) -> Self {
        self.into_table = Some(from.into());
        self
    }

    pub fn build(self) -> Self {
        self
    }
}

impl From<InsertQuery> for SQLStatement {
    fn from(value: InsertQuery) -> SQLStatement {
        SQLStatement::DML(DMLStatement::InsertQuery(value))
    }
}
