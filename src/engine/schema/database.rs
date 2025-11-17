use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DatabaseSchema {
    pub database_name: String,
}
