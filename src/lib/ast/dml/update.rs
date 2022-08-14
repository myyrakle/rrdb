use crate::lib::ast::predule::{DMLStatement, SQLStatement};

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateQuery {}

impl From<UpdateQuery> for SQLStatement {
    fn from(value: UpdateQuery) -> SQLStatement {
        SQLStatement::DML(DMLStatement::UpdateQuery(value))
    }
}
