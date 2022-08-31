pub struct ExecuteResult {
    pub rows: Vec<ExecuteRow>,
}

pub struct ExecuteRow {
    pub columns: Vec<ExecuteColumn>,
}

pub enum ExecuteColumn {
    Bool(bool),
    Integer(i128),
    Float(f64),
    Date(String),
    String(String),
}
