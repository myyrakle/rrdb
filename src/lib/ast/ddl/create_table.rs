pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};
pub use crate::lib::ast::types::Column;
use crate::lib::{CheckConstraint, ForeignKey, TableOptions};

struct CreateTableQuery {
    database_name: Option<String>,
    table_name: String,
    columns: Vec<Column>,
    primary_key: Option<Vec<String>>,
    foreign_keys: Vec<ForeignKey>,
    unique_keys: Vec<Vec<String>>,
    check_constraints: Vec<CheckConstraint>,
    table_options: TableOptions,
}

impl DDLStatement for CreateTableQuery {}

impl SQLStatement for CreateTableQuery {}
