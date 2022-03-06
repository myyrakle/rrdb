use serde::Deserialize;

#[derive(Deserialize)]
pub struct TableConfig {
    pub table_name: String,
}
