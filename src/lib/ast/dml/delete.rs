use crate::lib::ast::predule::{DMLStatement, SQLStatement};

#[derive(Clone, Debug, PartialEq)]
pub struct DeleteQuery {}

impl From<DeleteQuery> for SQLStatement {
    fn from(value: DeleteQuery) -> SQLStatement {
        SQLStatement::DML(DMLStatement::DeleteQuery(value))
    }
}
