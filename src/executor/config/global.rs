use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GlobalConfig {
    pub port: u32,
    pub host: String,
    pub data_directory: String,
}

#[allow(clippy::derivable_impls)]
impl std::default::Default for GlobalConfig {
    fn default() -> Self {
        Self {
            port: 55555,
            host: "0.0.0.0".to_string(),
            data_directory: "data".to_string(),
        }
    }
}
