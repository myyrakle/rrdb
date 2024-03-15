use crate::ast::predule::{OtherStatement, SQLStatement};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShowTablesQuery {
    pub database: String,
}

impl From<ShowTablesQuery> for SQLStatement {
    fn from(value: ShowTablesQuery) -> SQLStatement {
        SQLStatement::Other(OtherStatement::ShowTables(value))
    }
}
