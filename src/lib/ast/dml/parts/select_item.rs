use crate::lib::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct SelectItem {
    item: SQLExpression,   // select 요소
    alias: Option<String>, // as 절이 있을 경우 alias 정보
}
