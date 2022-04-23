pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};
use crate::lib::ast::types::Column;
use crate::lib::ast::types::Table;
use crate::lib::DataType;

/*
ALTER TABLE [database_name.]table_name
[RENAME COLUMN from_name TO to_name]
[ALTER COLUMN column_name TYPE]
[DROP COLUMN column_name]
[ADD COLUMN column_name column_type]
;
*/
#[derive(Debug, Clone)]
pub struct AlterTableQuery {
    pub table: Table,
    pub rename_to: Option<AlterTableRenameTo>,
    pub add_columns: Vec<AlterTableAddColumn>,
    pub alter_columns: Vec<AlterTableAlterColumn>,
    pub drop_columns: Vec<AlterTableDropColumn>,
    pub rename_columns: Vec<AlterTableRenameColumn>,
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
    pub set_type: Option<DataType>,
    pub set_default: Option<()>, // ... 아직 미설계
    pub drop_default: Option<bool>,
    pub set_not_null: Option<bool>,
    pub set_primary_key: Option<bool>,
    pub set_comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AlterTableRenameColumn {
    pub from_name: String,
    pub to_name: String,
}
