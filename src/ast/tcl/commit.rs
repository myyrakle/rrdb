use crate::ast::{SQLStatement, TCLStatement};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitQuery {}

impl From<CommitQuery> for SQLStatement {
    fn from(value: CommitQuery) -> SQLStatement {
        SQLStatement::TCL(TCLStatement::Commit(value))
    }
}
