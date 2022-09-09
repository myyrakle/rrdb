use crate::lib::ast::ddl::{
    AlterDatabaseQuery, AlterTableQuery, CreateDatabaseQuery, CreateTableQuery, DropDatabaseQuery,
    DropTableQuery,
};
use crate::lib::ast::dml::{DeleteQuery, InsertQuery, SelectQuery, UpdateQuery};
use crate::lib::ast::other::ShowDatabasesQuery;

#[derive(Clone, Debug, PartialEq)]
pub enum SQLStatement {
    DDL(DDLStatement),
    DML(DMLStatement),
    DCL(DCLStatement),
    Other(OtherStatement),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DDLStatement {
    CreateDatabaseQuery(CreateDatabaseQuery),
    AlterDatabase(AlterDatabaseQuery),
    DropDatabaseQuery(DropDatabaseQuery),
    CreateTableQuery(CreateTableQuery),
    AlterTableQuery(AlterTableQuery),
    DropTableQuery(DropTableQuery),
}

#[derive(Clone, Debug, PartialEq)]
pub enum DMLStatement {
    InsertQuery(InsertQuery),
    UpdateQuery(UpdateQuery),
    DeleteQuery(DeleteQuery),
    SelectQuery(SelectQuery),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DCLStatement {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OtherStatement {
    ShowDatabases(ShowDatabasesQuery),
}
