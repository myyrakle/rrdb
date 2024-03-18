use crate::ast::{SQLStatement, TCLStatement};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeginTransactionQuery {}

impl From<BeginTransactionQuery> for SQLStatement {
    fn from(value: BeginTransactionQuery) -> SQLStatement {
        SQLStatement::TCL(TCLStatement::BeginTransaction(value))
    }
}
