use crate::lib::ast::predule::TableName;

#[derive(Clone, Debug, PartialEq)]
pub struct SelectFromPlan {
    table_name: TableName,
    select_columns: Vec<String>,
    index: (),
    key: (),
    filter: (),
}
