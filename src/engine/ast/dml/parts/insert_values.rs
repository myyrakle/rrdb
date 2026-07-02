use crate::engine::ast::types::SQLExpression;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct InsertValue {
    pub list: Vec<Option<SQLExpression>>,
}
