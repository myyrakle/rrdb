use crate::ast::predule::TableName;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ForeignKey {
    pub key_name: String,
    pub table: TableName,
    pub columns: Vec<String>,
    pub referenced_table: TableName,
    pub referenced_columns: Vec<String>,
}
