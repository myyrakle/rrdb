use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GlobalConfig {
    pub databases: Vec<GlobalConfigDatabase>,
}

impl std::default::Default for GlobalConfig {
    fn default() -> Self {
        Self { databases: vec![] }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GlobalConfigDatabase {
    pub database_name: String,
}
