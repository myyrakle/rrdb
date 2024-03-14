use crate::ast::predule::{
    FilterPlan, GroupByClause, JoinPlan, LimitOffsetPlan, OrderByClause, SelectFromPlan,
    SelectSubqueryPlan,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectPlan {
    pub list: Vec<SelectPlanItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectPlanItem {
    From(SelectFromPlan),
    Subquery(SelectSubqueryPlan),
    Join(JoinPlan),
    Order(OrderByClause),
    Group(GroupByClause),
    GroupAll,
    LimitOffset(LimitOffsetPlan),
    Filter(FilterPlan),
}

impl From<SelectFromPlan> for SelectPlanItem {
    fn from(value: SelectFromPlan) -> SelectPlanItem {
        SelectPlanItem::From(value)
    }
}

impl From<SelectSubqueryPlan> for SelectPlanItem {
    fn from(value: SelectSubqueryPlan) -> SelectPlanItem {
        SelectPlanItem::Subquery(value)
    }
}

impl From<JoinPlan> for SelectPlanItem {
    fn from(value: JoinPlan) -> SelectPlanItem {
        SelectPlanItem::Join(value)
    }
}

impl From<OrderByClause> for SelectPlanItem {
    fn from(value: OrderByClause) -> SelectPlanItem {
        SelectPlanItem::Order(value)
    }
}

impl From<GroupByClause> for SelectPlanItem {
    fn from(value: GroupByClause) -> SelectPlanItem {
        SelectPlanItem::Group(value)
    }
}

impl From<LimitOffsetPlan> for SelectPlanItem {
    fn from(value: LimitOffsetPlan) -> SelectPlanItem {
        SelectPlanItem::LimitOffset(value)
    }
}

impl From<FilterPlan> for SelectPlanItem {
    fn from(value: FilterPlan) -> SelectPlanItem {
        SelectPlanItem::Filter(value)
    }
}
