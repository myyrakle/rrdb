use crate::lib::ast::predule::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct GroupByClause {
    pub group_by_items: Vec<GroupByItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupByItem {
    pub item: SQLExpression,
}
