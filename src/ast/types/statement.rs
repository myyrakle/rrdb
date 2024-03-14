use crate::ast::ddl::{
    AlterDatabaseQuery, AlterTableQuery, CreateDatabaseQuery, CreateIndexQuery, CreateTableQuery,
    DropDatabaseQuery, DropTableQuery,
};
use crate::ast::dml::{DeleteQuery, InsertQuery, SelectQuery, UpdateQuery};
use crate::ast::other::{DescTableQuery, ShowDatabasesQuery, ShowTablesQuery, UseDatabaseQuery};

#[derive(Clone, Debug, PartialEq)]
pub enum SQLStatement {
    DDL(DDLStatement),
    DML(DMLStatement),
    DCL(DCLStatement),
    Other(OtherStatement),
}

#[derive(Clone, Debug, PartialEq)]
pub enum DDLStatement {
    CreateDatabaseQuery(CreateDatabaseQuery),
    AlterDatabase(AlterDatabaseQuery),
    DropDatabaseQuery(DropDatabaseQuery),
    CreateTableQuery(CreateTableQuery),
    AlterTableQuery(AlterTableQuery),
    DropTableQuery(DropTableQuery),
    CreateIndexQuery(CreateIndexQuery),
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
    UseDatabase(UseDatabaseQuery),
    ShowTables(ShowTablesQuery),
    DescTable(DescTableQuery),
}
