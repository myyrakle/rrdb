use crate::lib::ast::predule::{SQLExpression, SelectColumn};

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateItem {
    column: SelectColumn,         // update할 컬럼
    value: Option<SQLExpression>, // 수정할 값
}
