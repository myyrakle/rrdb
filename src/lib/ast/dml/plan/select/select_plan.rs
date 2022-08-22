use super::{from::SelectFromPlan, SelectJoinPlan, SelectSubqueryPlan};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectPlan {
    list: Vec<SelectPlanItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectPlanItem {
    From(SelectFromPlan),
    Subquery(SelectSubqueryPlan),
    Join(SelectJoinPlan),
    Order,
    Group,
    Offset(u32),
    Limit(u32),
}
