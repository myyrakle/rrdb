use crate::lib::dml::parts::_where::WhereClause;
use crate::lib::{DMLStatement, GroupByClause, OrderByClause, SQLStatement, Table};

use super::SelectItem;

pub struct SelectQuery {
    select_items: Vec<SelectItem>,
    from_table: Option<Table>,
    where_clause: Option<WhereClause>,
    group_by_clause: Option<GroupByClause>,
    order_by_clause: Option<OrderByClause>,
    limit: Option<i32>,
    offset: Option<i32>,
}

impl DMLStatement for SelectQuery {}

impl SQLStatement for SelectQuery {}
