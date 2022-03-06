use serde::Deserialize;

#[derive(Deserialize)]
pub struct GlobalConfig {
    pub database_names: Vec<String>,
}
