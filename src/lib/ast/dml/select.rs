use crate::lib::ast::dml::parts::_where::WhereClause;
use crate::lib::ast::enums::{DMLStatement, SQLStatement};
use crate::lib::{GroupByClause, OrderByClause, TableName};

use super::SelectItem;

#[derive(Debug, Clone)]
pub struct SelectQuery {
    pub select_items: Vec<SelectItem>,
    pub from_table: Option<TableName>,
    pub where_clause: Option<WhereClause>,
    pub group_by_clause: Option<GroupByClause>,
    pub order_by_clause: Option<OrderByClause>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl SelectQuery {
    pub fn builder() -> Self {
        SelectQuery {
            select_items: vec![],
            from_table: None,
            where_clause: None,
            group_by_clause: None,
            order_by_clause: None,
            limit: None,
            offset: None,
        }
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DML(DMLStatement::SelectQuery(self))
    }
}
