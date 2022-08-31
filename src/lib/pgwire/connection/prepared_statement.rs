use crate::lib::ast::predule::SQLStatement;
use crate::lib::pgwire::protocol::FieldDescription;

#[derive(Debug, Clone)]
pub struct PreparedStatement {
    pub statement: Option<SQLStatement>,
    pub fields: Vec<FieldDescription>,
}
