pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};
use crate::lib::Table;
pub struct DropTableQuery {
    pub table: Table,
}

impl DDLStatement for DropTableQuery {}

impl SQLStatement for DropTableQuery {}
