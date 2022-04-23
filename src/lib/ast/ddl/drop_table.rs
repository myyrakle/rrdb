pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};
use crate::lib::Table;

/*
DROP TABLE [database_name.]table_name;
*/
#[derive(Debug, Clone)]
pub struct DropTableQuery {
    pub table: Option<Table>,
}

impl DropTableQuery {
    pub fn builder() -> Self {
        DropTableQuery { table: None }
    }

    pub fn set_table<'a>(&'a mut self, table: Table) -> &'a mut Self {
        self.table = Some(table);
        self
    }

    pub fn build(self) -> Box<dyn SQLStatement> {
        Box::new(self)
    }
}

impl DDLStatement for DropTableQuery {}

impl SQLStatement for DropTableQuery {}
