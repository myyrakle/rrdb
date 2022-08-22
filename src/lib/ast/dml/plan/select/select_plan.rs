#[derive(Clone, Debug, PartialEq)]
pub struct SelectPlan {
    list: Vec<SelectPlanItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectPlanItem {
    From,
    Subquery,
    Join,
    Order,
    Group,
    Offset(u32),
    Limit(u32),
}
