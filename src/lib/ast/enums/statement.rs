use crate::lib::ast::ddl::{
    AlterDatabaseQuery, AlterTableQuery, CreateDatabaseQuery, CreateTableQuery, DropDatabaseQuery,
    DropTableQuery,
};
use crate::lib::ast::dml::{DeleteQuery, InsertQuery, SelectQuery, UpdateQuery};

#[derive(Clone, Debug)]
pub enum SQLStatement {
    DDL(DDLStatement),
    DML(DMLStatement),
    DCL(DCLStatement),
}

#[derive(Clone, Debug)]
pub enum DDLStatement {
    CreateDatabaseQuery(CreateDatabaseQuery),
    AlterDatabase(AlterDatabaseQuery),
    DropDatabaseQuery(DropDatabaseQuery),
    CreateTableQuery(CreateTableQuery),
    AlterTableQuery(AlterTableQuery),
    DropTableQuery(DropTableQuery),
}

#[derive(Clone, Debug)]
pub enum DMLStatement {
    InsertQuery(InsertQuery),
    UpdateQuery(UpdateQuery),
    DeleteQuery(DeleteQuery),
    SelectQuery(SelectQuery),
}

#[derive(Clone, Debug)]
pub enum DCLStatement {}
