use crate::lib::ast::ddl::{
    AlterDatabaseQuery, AlterTableQuery, CreateDatabaseQuery, CreateTableQuery, DropDatabaseQuery,
    DropTableQuery,
};
use crate::lib::ast::dml::{DeleteQuery, InsertQuery, SelectQuery, UpdateQuery};
use crate::lib::ast::predule::{FromClause, FromTarget};

#[derive(Clone, Debug, PartialEq)]
pub enum SQLStatement {
    DDL(DDLStatement),
    DML(DMLStatement),
    DCL(DCLStatement),
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub enum DCLStatement {}

impl From<SQLStatement> for FromClause {
    fn from(value: SQLStatement) -> FromClause {
        FromClause {
            from: FromTarget::Subquery(Box::new(value)),
            alias: None,
        }
    }
}
