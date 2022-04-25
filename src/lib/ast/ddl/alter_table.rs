//use crate::lib::ast::enums::SQLStatement;
use crate::lib::ast::types::Column;
use crate::lib::ast::types::Table;
use crate::lib::DataType;

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
#[derive(Debug, Clone)]
pub struct AlterTableQuery {
    pub table: Table,
    pub action: AlterTableAction,
}

#[derive(Debug, Clone)]
pub enum AlterTableAction {
    AlterTableRenameTo(AlterTableRenameTo),
    AddColumn(AlterTableAddColumn),
    AlterColumn(AlterTableAlterColumn),
    DropColumn(AlterTableDropColumn),
    RenameColumn(AlterTableRenameColumn),
}

// 테이블명 변경
// ALTER TABLE [database_name.]table_name RENAME TO new_table_name;
#[derive(Debug, Clone)]
pub struct AlterTableRenameTo {
    pub name: String,
}

// 컬럼 이름 변경
// ALTER TABLE [database_name.]table_name RENAME COLUMN from_name TO new_name;
#[derive(Debug, Clone)]
pub struct AlterTableRenameColumn {
    pub from_name: String,
    pub to_name: String,
}

// 컬럼 추가
// ALTER TABLE [database_name.]table_name ADD COLUMN column_name column_type [NOT NULL | NULL] [PRIMARY KEY] [COMMENT 'comment'];
#[derive(Debug, Clone)]
pub struct AlterTableAddColumn {
    pub column: Column,
}

// 컬럼 삭제
// ALTER TABLE [database_name.]table_name DROP COLUMN column_name;
#[derive(Debug, Clone)]
pub struct AlterTableDropColumn {
    pub column_name: String,
}

// 컬럼 변경
// ALTER COLUMN column_name [TYPE type_name] [{SET | DROP} NOT NULL] [{SET | DROP} DEFAULT default_expr] [{SET | DROP} COMMENT 'comment']
#[derive(Debug, Clone)]
pub struct AlterTableAlterColumn {
    pub column_name: String,
    pub action: AlterColumnAction,
}

#[derive(Debug, Clone)]
pub enum AlterColumnAction {
    AlterColumnType(DataType),
    AlterColumnSetNotNull,
    AlterColumnDropNotNull,
    AlterColumnSetDefault(AlterTableSetDefault),
    AlterColumnDropDefault(AlterTableDropDefault),
}

#[derive(Debug, Clone)]
pub struct AlterTableSetType {
    pub data_type: DataType,
}

#[derive(Debug, Clone)]
pub struct AlterColumnSetNotNull {
    pub data_type: DataType,
}

#[derive(Debug, Clone)]
pub struct AlterColumnDropNotNull {
    pub data_type: DataType,
}

#[derive(Debug, Clone)]
pub struct AlterTableSetDefault {
    pub default_expression: (),
}

#[derive(Debug, Clone)]
pub struct AlterTableDropDefault {}
