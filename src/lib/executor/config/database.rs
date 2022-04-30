use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DatabaseConfig {
    pub database_name: String,
}
