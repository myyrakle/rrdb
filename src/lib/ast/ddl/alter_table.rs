pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};

struct AlterTable {
    database_name: String,
    table_name: String,
    // columns: Vec<Column>,
    // primary_key: Option<Vec<String>>,
    // foreign_keys: Vec<ForeignKey>,
    // unique_keys: Vec<Vec<String>>,
    // check_constraints: Vec<CheckConstraint>,
    // table_options: TableOptions,
}

impl DDLStatement for AlterTable {}

impl SQLStatement for AlterTable {}
