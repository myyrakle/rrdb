use crate::ast::predule::SelectColumn;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct GroupByClause {
    pub group_by_items: Vec<GroupByItem>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GroupByItem {
    pub item: SelectColumn,
}
