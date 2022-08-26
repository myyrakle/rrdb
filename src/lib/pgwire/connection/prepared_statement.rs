#[derive(Debug, Clone)]
pub struct PreparedStatement {
    pub statement: Option<Statement>,
    pub fields: Vec<FieldDescription>,
}
