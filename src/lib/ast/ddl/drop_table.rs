pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};

struct DropTableQuery {
    database_name: String,
    table_name: String,
}

impl DDLStatement for DropTableQuery {}

impl SQLStatement for DropTableQuery {}
