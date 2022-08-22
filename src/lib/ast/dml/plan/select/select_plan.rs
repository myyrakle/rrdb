use crate::lib::ast::predule::{
    GroupByClause, OrderByClause, SelectFromPlan, SelectJoinPlan, SelectSubqueryPlan,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectPlan {
    list: Vec<SelectPlanItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectPlanItem {
    From(SelectFromPlan),
    Subquery(SelectSubqueryPlan),
    Join(SelectJoinPlan),
    Order(OrderByClause),
    Group(GroupByClause),
    Offset(u32),
    Limit(u32),
}
