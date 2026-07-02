use crate::engine::ast::types::SQLExpression;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct UpdateItem {
    pub column: String,       // update할 컬럼
    pub value: SQLExpression, // 수정할 값
}
