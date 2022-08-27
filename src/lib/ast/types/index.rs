#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub index_name: String,
    pub database_name: Option<String>,
    pub columns: Vec<String>,
}
