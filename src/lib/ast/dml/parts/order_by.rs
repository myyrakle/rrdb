use crate::lib::ast::predule::SQLExpression;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OrderByClause {
    pub order_by_items: Vec<OrderByItem>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct OrderByItem {
    pub item: SQLExpression,
    pub order_type: OrderByType,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum OrderByType {
    Asc,
    Desc,
}
