#[derive(Clone, Debug)]
pub struct UniqueKey {
    pub key_name: String,
    pub database_name: Option<String>,
    pub columns: Vec<String>,
}
