use crate::ast::predule::SQLExpression;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OrderByClause {
    pub order_by_items: Vec<OrderByItem>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OrderByItem {
    pub item: SQLExpression,
    pub order_type: OrderByType,
    pub nulls: OrderByNulls,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum OrderByType {
    Asc,
    Desc,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum OrderByNulls {
    First,
    Last,
}
