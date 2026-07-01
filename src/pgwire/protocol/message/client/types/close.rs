#[derive(Debug)]
pub enum Close {
    Portal(String),
    PreparedStatement(String),
}
