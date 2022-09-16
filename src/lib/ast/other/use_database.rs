use crate::lib::ast::predule::{OtherStatement, SQLStatement};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseDatabaseQuery {
    database_name: String,
}

impl From<UseDatabaseQuery> for SQLStatement {
    fn from(value: UseDatabaseQuery) -> SQLStatement {
        SQLStatement::Other(OtherStatement::UseDatabase(value))
    }
}
