pub mod commands;
pub mod dcl;
pub mod ddl;
pub mod dml;
pub mod other;
pub mod tcl;
pub mod types;

use crate::engine::ast::{
    ddl::{
        alter_database::AlterDatabaseQuery, alter_table::AlterTableQuery,
        create_database::CreateDatabaseQuery, create_index::CreateIndexQuery,
        create_table::CreateTableQuery, drop_database::DropDatabaseQuery,
        drop_table::DropTableQuery,
    },
    dml::{delete::DeleteQuery, insert::InsertQuery, select::SelectQuery, update::UpdateQuery},
    other::{
        desc_table::DescTableQuery, show_databases::ShowDatabasesQuery,
        show_tables::ShowTablesQuery, use_database::UseDatabaseQuery,
    },
};

use self::tcl::{BeginTransactionQuery, CommitQuery, RollbackQuery};

#[derive(Clone, Debug, PartialEq, Default)]
pub enum SQLStatement {
    DDL(DDLStatement),
    DML(DMLStatement),
    DCL(DCLStatement),
    TCL(TCLStatement),
    Other(OtherStatement),
    #[default]
    None,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TCLStatement {
    BeginTransaction(BeginTransactionQuery),
    Commit(CommitQuery),
    Rollback(RollbackQuery),
}
