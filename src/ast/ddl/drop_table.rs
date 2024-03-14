use crate::ast::predule::{DDLStatement, SQLStatement, TableName};

/*
DROP TABLE [IF EXISTS] [database_name.]table_name;
*/
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropTableQuery {
    pub table: Option<TableName>,
    pub if_exists: bool,
}

impl DropTableQuery {
    pub fn builder() -> Self {
        DropTableQuery {
            table: None,
            if_exists: false,
        }
    }

    pub fn set_table(mut self, table: TableName) -> Self {
        self.table = Some(table);
        self
    }

    pub fn set_if_exists(mut self, set_if_exists: bool) -> Self {
        self.if_exists = set_if_exists;
        self
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DDL(DDLStatement::DropTableQuery(self))
    }
}
