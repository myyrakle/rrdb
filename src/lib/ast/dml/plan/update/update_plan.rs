use crate::lib::ast::{dml::ScanType, predule::FilterPlan};

#[derive(Clone, Debug, PartialEq)]
pub struct UpdatePlan {
    pub list: Vec<UpdatePlanItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpdatePlanItem {
    UpdateScan(ScanType),
    Filter(FilterPlan),
}

impl From<FilterPlan> for UpdatePlanItem {
    fn from(value: FilterPlan) -> UpdatePlanItem {
        UpdatePlanItem::Filter(value)
    }
}

impl From<ScanType> for UpdatePlanItem {
    fn from(value: ScanType) -> UpdatePlanItem {
        UpdatePlanItem::UpdateScan(value)
    }
}
