//use crate::ast::enums::SQLStatement;
use crate::ast::predule::{Column, DataType, SQLExpression, SQLStatement, TableName};

use super::DDLStatement;

/*
ALTER TABLE [database_name.]table_name
{
    [RENAME TO new_table_name] |
    [RENAME COLUMN from_name TO new_name] |
    [ALTER COLUMN column_name ...] |
    [DROP COLUMN column_name] |
    [ADD COLUMN column_name column_type ... ] ...
};
*/
#[derive(Clone, Debug, PartialEq)]
pub struct AlterTableQuery {
    pub table: Option<TableName>,
    pub action: AlterTableAction,
}

impl AlterTableQuery {
    pub fn builder() -> Self {
        AlterTableQuery {
            table: None,
            action: AlterTableAction::None,
        }
    }

    pub fn set_table(mut self, table: TableName) -> Self {
        self.table = Some(table);
        self
    }

    pub fn set_action(mut self, action: AlterTableAction) -> Self {
        self.action = action;
        self
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DDL(DDLStatement::AlterTableQuery(self))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AlterTableAction {
    AlterTableRenameTo(AlterTableRenameTo),
    AddColumn(AlterTableAddColumn),
    AlterColumn(AlterTableAlterColumn),
    DropColumn(AlterTableDropColumn),
    RenameColumn(AlterTableRenameColumn),
    None,
}

// 테이블명 변경
// ALTER TABLE [database_name.]table_name RENAME TO new_table_name;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterTableRenameTo {
    pub name: String,
}

impl From<AlterTableRenameTo> for AlterTableAction {
    fn from(value: AlterTableRenameTo) -> AlterTableAction {
        AlterTableAction::AlterTableRenameTo(value)
    }
}

// 컬럼 이름 변경
// ALTER TABLE [database_name.]table_name RENAME COLUMN from_name TO new_name;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterTableRenameColumn {
    pub from_name: String,
    pub to_name: String,
}

impl From<AlterTableRenameColumn> for AlterTableAction {
    fn from(value: AlterTableRenameColumn) -> AlterTableAction {
        AlterTableAction::RenameColumn(value)
    }
}

// 컬럼 추가
// ALTER TABLE [database_name.]table_name ADD COLUMN column_name column_type [NOT NULL | NULL] [PRIMARY KEY] [COMMENT 'comment'];
#[derive(Clone, Debug, PartialEq)]
pub struct AlterTableAddColumn {
    pub column: Column,
}

impl From<AlterTableAddColumn> for AlterTableAction {
    fn from(value: AlterTableAddColumn) -> AlterTableAction {
        AlterTableAction::AddColumn(value)
    }
}

// 컬럼 삭제
// ALTER TABLE [database_name.]table_name DROP COLUMN column_name;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterTableDropColumn {
    pub column_name: String,
}

impl From<AlterTableDropColumn> for AlterTableAction {
    fn from(value: AlterTableDropColumn) -> AlterTableAction {
        AlterTableAction::DropColumn(value)
    }
}

// 컬럼 변경
// ALTER COLUMN column_name [TYPE type_name] [{SET | DROP} NOT NULL] [{SET | DROP} DEFAULT default_expr] [{SET | DROP} COMMENT 'comment']
#[derive(Clone, Debug, PartialEq)]
pub struct AlterTableAlterColumn {
    pub column_name: String,
    pub action: AlterColumnAction,
}

impl From<AlterTableAlterColumn> for AlterTableAction {
    fn from(value: AlterTableAlterColumn) -> AlterTableAction {
        AlterTableAction::AlterColumn(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AlterColumnAction {
    AlterColumnSetType(AlterColumnSetType),
    AlterColumnSetNotNull,
    AlterColumnDropNotNull,
    AlterColumnSetDefault(AlterColumnSetDefault),
    AlterColumnDropDefault(AlterColumnDropDefault),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterColumnSetType {
    pub data_type: DataType,
}

impl From<AlterColumnSetType> for AlterColumnAction {
    fn from(value: AlterColumnSetType) -> AlterColumnAction {
        AlterColumnAction::AlterColumnSetType(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterColumnSetNotNull {}

impl From<AlterColumnSetNotNull> for AlterColumnAction {
    fn from(_value: AlterColumnSetNotNull) -> AlterColumnAction {
        AlterColumnAction::AlterColumnSetNotNull
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterColumnDropNotNull {}

impl From<AlterColumnDropNotNull> for AlterColumnAction {
    fn from(_value: AlterColumnDropNotNull) -> AlterColumnAction {
        AlterColumnAction::AlterColumnDropNotNull
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlterColumnSetDefault {
    pub expression: SQLExpression,
}

impl From<AlterColumnSetDefault> for AlterColumnAction {
    fn from(value: AlterColumnSetDefault) -> AlterColumnAction {
        AlterColumnAction::AlterColumnSetDefault(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterColumnDropDefault {}

impl From<AlterColumnDropDefault> for AlterColumnAction {
    fn from(value: AlterColumnDropDefault) -> AlterColumnAction {
        AlterColumnAction::AlterColumnDropDefault(value)
    }
}
