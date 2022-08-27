#[derive(Debug)]
pub enum Describe {
    Portal(String),
    PreparedStatement(String),
}
