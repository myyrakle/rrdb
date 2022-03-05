pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};
pub use crate::lib::ast::types::Column;
use crate::lib::{CheckConstraint, ForeignKey, TableOptions};

struct CreateTable {
    database_name: Option<String>,
    table_name: String,
    columns: Vec<Column>,
    primary_key: Option<Vec<String>>,
    foreign_keys: Vec<ForeignKey>,
    unique_keys: Vec<Vec<String>>,
    check_constraints: Vec<CheckConstraint>,
    table_options: TableOptions,
}

impl DDLStatement for CreateTable {}

impl SQLStatement for CreateTable {}
