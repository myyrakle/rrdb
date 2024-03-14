use crate::ast::predule::SQLStatement;
use crate::pgwire::protocol::FieldDescription;

#[derive(Debug, Clone)]
pub struct PreparedStatement {
    pub statement: Option<SQLStatement>,
    pub fields: Vec<FieldDescription>,
}
