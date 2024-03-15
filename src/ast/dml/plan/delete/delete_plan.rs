use crate::ast::dml::plan::select::filter::FilterPlan;

use super::from::DeleteFromPlan;

#[derive(Clone, Debug, PartialEq)]
pub struct DeletePlan {
    pub list: Vec<DeletePlanItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeletePlanItem {
    DeleteFrom(DeleteFromPlan),
    Filter(FilterPlan),
}

impl From<FilterPlan> for DeletePlanItem {
    fn from(value: FilterPlan) -> DeletePlanItem {
        DeletePlanItem::Filter(value)
    }
}

impl From<DeleteFromPlan> for DeletePlanItem {
    fn from(value: DeleteFromPlan) -> DeletePlanItem {
        DeletePlanItem::DeleteFrom(value)
    }
}
