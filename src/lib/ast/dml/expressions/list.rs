use crate::lib::ast::predule::SQLExpression;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ListExpression {
    pub value: Vec<SQLExpression>,
}

impl From<ListExpression> for SQLExpression {
    fn from(value: ListExpression) -> SQLExpression {
        SQLExpression::List(value)
    }
}
