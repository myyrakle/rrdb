use crate::engine::ast::SQLStatement;
use crate::pgwire::protocol::backend::FieldDescription;

#[derive(Debug, Clone)]
pub struct PreparedStatement {
    pub statement: Option<SQLStatement>,
    pub raw_query: Option<String>,
    pub fields: Vec<FieldDescription>,
}
