use crate::ast::types::SQLExpression;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Default)]
pub struct HavingClause {
    pub expression: Box<SQLExpression>,
}
