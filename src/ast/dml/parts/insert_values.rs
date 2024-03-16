use crate::ast::types::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct InsertValue {
    pub list: Vec<Option<SQLExpression>>,
}
