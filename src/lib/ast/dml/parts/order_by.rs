use crate::lib::ast::predule::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct OrderByClause {
    pub expression: Box<SQLExpression>,
    pub order_type: OrderByType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrderByType {
    AST,
    DESC,
}
