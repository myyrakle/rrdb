use crate::engine::ast::{OtherStatement, SQLStatement};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShowDatabasesQuery {}

impl From<ShowDatabasesQuery> for SQLStatement {
    fn from(value: ShowDatabasesQuery) -> SQLStatement {
        SQLStatement::Other(OtherStatement::ShowDatabases(value))
    }
}
