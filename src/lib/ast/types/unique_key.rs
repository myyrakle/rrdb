use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct UniqueKey {
    pub key_name: String,
    pub database_name: Option<String>,
    pub columns: Vec<String>,
}
