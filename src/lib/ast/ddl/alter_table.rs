pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};
use crate::lib::ast::types::Column;
use crate::lib::ast::types::Table;

#[derive(Debug, Clone)]
pub struct AlterTableQuery {
    pub table: Table,
    pub rename_to: Option<AlterTableRenameTo>,
    pub add_columns: Vec<AlterTableAddColumn>,
    pub alter_columns: Vec<AlterTableAlterColumn>,
    pub drop_columns: Vec<AlterTableDropColumn>,
}

impl DDLStatement for AlterTableQuery {}

impl SQLStatement for AlterTableQuery {}

#[derive(Debug, Clone)]
pub struct AlterTableRenameTo {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct AlterTableAddColumn {
    pub column: Column,
}

#[derive(Debug, Clone)]
pub struct AlterTableDropColumn {
    pub column_name: String,
}

#[derive(Debug, Clone)]
pub struct AlterTableAlterColumn {
    pub column_name: String,
}
