#[derive(Clone, Debug, PartialEq)]
pub struct Index {
    pub index_name: String,
    pub database_name: Option<String>,
    pub columns: Vec<String>,
}
