use crate::lib::ast::predule::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct InsertValue {
    list: Vec<SQLExpression>,
}
