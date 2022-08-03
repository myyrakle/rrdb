use crate::lib::ast::predule::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct ListExpression {
    pub value: Vec<SQLExpression>,
}
