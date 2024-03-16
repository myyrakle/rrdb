use crate::ast::{types::TableName, OtherStatement, SQLStatement};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescTableQuery {
    pub table_name: TableName,
}

impl From<DescTableQuery> for SQLStatement {
    fn from(value: DescTableQuery) -> SQLStatement {
        SQLStatement::Other(OtherStatement::DescTable(value))
    }
}
