use crate::ast::predule::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateItem {
    pub column: String,       // update할 컬럼
    pub value: SQLExpression, // 수정할 값
}
