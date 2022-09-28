use crate::lib::ast::predule::SelectColumn;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct GroupByClause {
    pub group_by_items: Vec<GroupByItem>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct GroupByItem {
    pub item: SelectColumn,
}
