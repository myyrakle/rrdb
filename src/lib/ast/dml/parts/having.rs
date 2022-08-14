use crate::lib::ast::predule::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct HavingClause {
    pub expression: Box<SQLExpression>,
}
