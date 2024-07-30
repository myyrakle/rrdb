use crate::ast::types::SQLExpression;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OrderByClause {
    pub order_by_items: Vec<OrderByItem>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Default)]
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

impl Default for OrderByType {
    fn default() -> Self {
        OrderByType::Asc
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum OrderByNulls {
    First,
    Last,
}

impl Default for OrderByNulls {
    fn default() -> Self {
        OrderByNulls::Last
    }
}
