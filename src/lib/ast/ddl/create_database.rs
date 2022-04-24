pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};
pub use crate::lib::ast::types::Column;

/*
CREATE DATABASE [IF NOT EXISTS] database_name;
*/
#[derive(Debug, Clone)]
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

    pub fn set_name<'a>(&'a mut self, name: String) -> &'a mut Self {
        self.database_name = Some(name);
        self
    }

    pub fn set_if_not_exists<'a>(&'a mut self, if_not_exists: bool) -> &'a mut Self {
        self.if_not_exists = if_not_exists;
        self
    }

    pub fn build(self) -> Box<dyn SQLStatement> {
        Box::new(self)
    }
}

impl DDLStatement for CreateDatabaseQuery {}

impl SQLStatement for CreateDatabaseQuery {}
