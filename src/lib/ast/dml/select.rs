use crate::lib::dml::parts::_where::WhereClause;
use crate::lib::{DMLStatement, GroupByClause, OrderByClause, SQLStatement, Table};

use super::SelectItem;

pub struct SelectQuery {
    pub select_items: Vec<SelectItem>,
    pub from_table: Option<Table>,
    pub where_clause: Option<WhereClause>,
    pub group_by_clause: Option<GroupByClause>,
    pub order_by_clause: Option<OrderByClause>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl DMLStatement for SelectQuery {}

impl SQLStatement for SelectQuery {}
