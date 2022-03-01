pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};

struct DropTable {
    database_name: String,
    table_name: String,
}

impl DDLStatement for DropTable {}

impl SQLStatement for DropTable {}
