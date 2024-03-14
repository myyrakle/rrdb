use crate::ast::predule::FilterPlan;

use super::DeleteFromPlan;

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
