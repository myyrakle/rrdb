use crate::ast::predule::TableName;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct UpdateTarget {
    pub table: TableName,
    pub alias: Option<String>,
}

impl From<TableName> for UpdateTarget {
    fn from(value: TableName) -> UpdateTarget {
        UpdateTarget {
            table: value,
            alias: None,
        }
    }
}
