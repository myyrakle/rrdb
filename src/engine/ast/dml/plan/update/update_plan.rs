use crate::engine::ast::dml::plan::select::filter::FilterPlan;

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

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::engine::ast::{
        dml::plan::select::scan::ScanType,
        types::{SQLExpression, TableName},
    };

    use super::*;

    #[test]
    fn From_FilterPlan_for_UpdatePlanItem() {
        use super::UpdatePlanItem;

        let filter = FilterPlan {
            expression: SQLExpression::String("a".into()),
        };
        let update_plan_item: UpdatePlanItem = filter.clone().into();
        assert_eq!(update_plan_item, UpdatePlanItem::Filter(filter));
    }

    #[test]
    fn From_UpdateFromPlan_for_UpdatePlanItem() {
        use super::UpdateFromPlan;

        let update_from = UpdateFromPlan {
            table_name: TableName::new(None, "table".into()),
            alias: None,
            scan: ScanType::FullScan,
        };
        let update_plan_item: UpdatePlanItem = update_from.clone().into();
        assert_eq!(update_plan_item, UpdatePlanItem::UpdateFrom(update_from));
    }
}
