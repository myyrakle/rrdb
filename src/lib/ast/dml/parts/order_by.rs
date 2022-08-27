use crate::lib::ast::predule::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct OrderByClause {
    pub order_by_items: Vec<OrderByItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrderByItem {
    pub item: SQLExpression,
    pub order_type: OrderByType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderByType {
    Asc,
    Desc,
}
