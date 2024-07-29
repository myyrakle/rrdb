use crate::ast::types::SQLExpression;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Default)]
pub struct WhereClause {
    pub expression: SQLExpression,
}
