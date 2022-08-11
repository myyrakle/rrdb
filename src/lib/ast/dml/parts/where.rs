use crate::lib::ast::predule::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct WhereClause {
    pub expression: Option<Box<SQLExpression>>,
}
