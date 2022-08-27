use sqlparser::ast::Statement;

use crate::lib::pgwire::protocol::FieldDescription;

#[derive(Debug, Clone)]
pub struct PreparedStatement {
    pub statement: Option<Statement>,
    pub fields: Vec<FieldDescription>,
}
