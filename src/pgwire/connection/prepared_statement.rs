use crate::{ast::SQLStatement, pgwire::protocol::backend::FieldDescription};

#[derive(Debug, Clone)]
pub struct PreparedStatement {
    pub statement: Option<SQLStatement>,
    pub fields: Vec<FieldDescription>,
}
