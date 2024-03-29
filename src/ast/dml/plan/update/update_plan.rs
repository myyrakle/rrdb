use crate::ast::dml::plan::select::filter::FilterPlan;

use super::from::UpdateFromPlan;

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
