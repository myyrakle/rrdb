use serde::Deserialize;

#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub database_name: String,
    pub table_names: Vec<String>,
}
