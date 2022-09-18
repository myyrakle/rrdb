//use crate::lib::ast::enums::SQLStatement;
use crate::lib::ast::predule::DataType;
use crate::lib::ast::types::Column;
use crate::lib::ast::types::TableName;

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
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

// 컬럼 이름 변경
// ALTER TABLE [database_name.]table_name RENAME COLUMN from_name TO new_name;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterTableRenameColumn {
    pub from_name: String,
    pub to_name: String,
}

// 컬럼 추가
// ALTER TABLE [database_name.]table_name ADD COLUMN column_name column_type [NOT NULL | NULL] [PRIMARY KEY] [COMMENT 'comment'];
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterTableAddColumn {
    pub column: Column,
}

// 컬럼 삭제
// ALTER TABLE [database_name.]table_name DROP COLUMN column_name;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterTableDropColumn {
    pub column_name: String,
}

// 컬럼 변경
// ALTER COLUMN column_name [TYPE type_name] [{SET | DROP} NOT NULL] [{SET | DROP} DEFAULT default_expr] [{SET | DROP} COMMENT 'comment']
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterTableAlterColumn {
    pub column_name: String,
    pub action: AlterColumnAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlterColumnAction {
    AlterColumnType(DataType),
    AlterColumnSetNotNull,
    AlterColumnDropNotNull,
    AlterColumnSetDefault(AlterTableSetDefault),
    AlterColumnDropDefault(AlterTableDropDefault),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterTableSetType {
    pub data_type: DataType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterColumnSetNotNull {
    pub data_type: DataType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterColumnDropNotNull {
    pub data_type: DataType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterTableSetDefault {
    pub default_expression: (),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlterTableDropDefault {}
