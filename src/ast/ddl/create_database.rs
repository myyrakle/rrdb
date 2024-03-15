pub use crate::ast::predule::{Column, DDLStatement, SQLStatement};

/*
CREATE DATABASE [IF NOT EXISTS] database_name;
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateDatabaseQuery {
    pub database_name: Option<String>,
    pub if_not_exists: bool,
}

impl CreateDatabaseQuery {
    pub fn builder() -> Self {
        CreateDatabaseQuery {
            database_name: None,
            if_not_exists: false,
        }
    }

    pub fn set_name(mut self, name: String) -> Self {
        self.database_name = Some(name);
        self
    }

    pub fn set_if_not_exists(mut self, if_not_exists: bool) -> Self {
        self.if_not_exists = if_not_exists;
        self
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DDL(DDLStatement::CreateDatabaseQuery(self))
    }
}
