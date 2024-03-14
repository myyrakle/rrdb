use crate::ast::{
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
