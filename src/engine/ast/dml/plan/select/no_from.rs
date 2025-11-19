use crate::engine::ast::predule::{Index, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectNoFromPlan {
    select_columns: Vec<String>,
}
