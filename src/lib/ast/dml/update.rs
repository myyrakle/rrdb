use crate::lib::ast::predule::{DMLStatement, SQLStatement, TableName, WhereClause, UpdateItem};

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateQuery {
    pub target_table: Option<TableName>,
    pub where_clause: Option<WhereClause>,
    pub update_items: Vec<UpdateItem>,
}

impl From<UpdateQuery> for SQLStatement {
    fn from(value: UpdateQuery) -> SQLStatement {
        SQLStatement::DML(DMLStatement::UpdateQuery(value))
    }
}
