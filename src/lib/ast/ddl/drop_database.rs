use crate::lib::ast::enums::{DDLStatement, SQLStatement};
pub use crate::lib::ast::types::Column;

/*
DROP DATABASE [IF EXISTS] database_name;
*/
#[derive(Debug, Clone)]
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

    pub fn set_name<'a>(&'a mut self, name: String) -> &'a mut Self {
        self.database_name = Some(name);
        self
    }

    pub fn set_if_exists<'a>(&'a mut self, set_if_exists: bool) -> &'a mut Self {
        self.if_exists = set_if_exists;
        self
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DDL(DDLStatement::DropDatabaseQuery(self))
    }
}
