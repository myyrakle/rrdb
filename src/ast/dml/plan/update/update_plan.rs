use crate::ast::predule::FilterPlan;

use super::UpdateFromPlan;

#[derive(Clone, Debug, PartialEq)]
pub struct UpdatePlan {
    pub list: Vec<UpdatePlanItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpdatePlanItem {
    UpdateFrom(UpdateFromPlan),
    Filter(FilterPlan),
}

impl From<FilterPlan> for UpdatePlanItem {
    fn from(value: FilterPlan) -> UpdatePlanItem {
        UpdatePlanItem::Filter(value)
    }
}

impl From<UpdateFromPlan> for UpdatePlanItem {
    fn from(value: UpdateFromPlan) -> UpdatePlanItem {
        UpdatePlanItem::UpdateFrom(value)
    }
}
