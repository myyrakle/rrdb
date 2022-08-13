use crate::lib::ast::predule::SQLExpression;

// [table_alias.]column_name
// SELECT시 컬럼 지정을 가리키는 값입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectColumn {
    pub table_name: Option<String>,
    pub column_name: String,
}

impl SelectColumn {
    pub fn new(table_name: Option<String>, column_name: String) -> Self {
        SelectColumn {
            column_name,
            table_name,
        }
    }
}

impl From<SelectColumn> for SQLExpression {
    fn from(value: SelectColumn) -> SQLExpression {
        SQLExpression::SelectColumn(value)
    }
}
