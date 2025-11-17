use crate::engine::ast::{SQLStatement, TCLStatement};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollbackQuery {}

impl From<RollbackQuery> for SQLStatement {
    fn from(value: RollbackQuery) -> SQLStatement {
        SQLStatement::TCL(TCLStatement::Rollback(value))
    }
}
