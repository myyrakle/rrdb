#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteResult {
    pub rows: Vec<ExecuteRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteRow {
    pub columns: Vec<ExecuteColumn>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExecuteColumn {
    Bool(bool),
    Integer(i128),
    Float(f64),
    Date(String),
    String(String),
}
