pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};
use crate::lib::Table;

/*
DROP TABLE [IF EXISTS] [database_name.]table_name;
*/
#[derive(Debug, Clone)]
pub struct DropTableQuery {
    pub table: Option<Table>,
    pub if_exists: bool,
}

impl DropTableQuery {
    pub fn builder() -> Self {
        DropTableQuery {
            table: None,
            if_exists: false,
        }
    }

    pub fn set_table<'a>(&'a mut self, table: Table) -> &'a mut Self {
        self.table = Some(table);
        self
    }

    pub fn set_if_exists<'a>(&'a mut self, set_if_exists: bool) -> &'a mut Self {
        self.if_exists = set_if_exists;
        self
    }

    pub fn build(self) -> Box<dyn SQLStatement> {
        Box::new(self)
    }
}

impl DDLStatement for DropTableQuery {}

impl SQLStatement for DropTableQuery {}
