use crate::lib::ast::predule::TableName;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct UpdateTarget {
    pub table: TableName,
    pub alias: Option<String>,
}
