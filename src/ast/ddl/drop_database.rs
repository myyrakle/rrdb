pub use crate::ast::predule::{Column, DDLStatement, SQLStatement};

/*
DROP DATABASE [IF EXISTS] database_name;
*/
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropDatabaseQuery {
    pub database_name: Option<String>,
    pub if_exists: bool,
}

impl DropDatabaseQuery {
    pub fn builder() -> Self {
        DropDatabaseQuery {
            database_name: None,
            if_exists: false,
        }
    }

    pub fn set_name(mut self, name: String) -> Self {
        self.database_name = Some(name);
        self
    }

    pub fn set_if_exists(mut self, set_if_exists: bool) -> Self {
        self.if_exists = set_if_exists;
        self
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DDL(DDLStatement::DropDatabaseQuery(self))
    }
}
