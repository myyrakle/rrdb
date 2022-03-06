pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};
pub use crate::lib::ast::types::Column;
use crate::lib::{CheckConstraint, ForeignKey, Table, TableOptions};

pub struct CreateTableQuery {
    pub table: Table,
    pub columns: Vec<Column>,
    pub primary_key: Option<Vec<String>>,
    pub foreign_keys: Vec<ForeignKey>,
    pub unique_keys: Vec<Vec<String>>,
    pub check_constraints: Vec<CheckConstraint>,
    pub table_options: TableOptions,
}

impl DDLStatement for CreateTableQuery {}

impl SQLStatement for CreateTableQuery {}
