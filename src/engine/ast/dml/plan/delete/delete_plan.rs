use crate::engine::ast::dml::plan::select::filter::FilterPlan;

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

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::engine::ast::{
        dml::plan::select::scan::ScanType,
        types::{SQLExpression, TableName},
    };

    use super::*;

    #[test]
    fn From_FilterPlan_for_DeletePlanItem() {
        use super::DeletePlanItem;

        let filter = FilterPlan {
            expression: SQLExpression::String("a".into()),
        };
        let delete_plan_item: DeletePlanItem = filter.clone().into();
        assert_eq!(delete_plan_item, DeletePlanItem::Filter(filter));
    }

    #[test]
    fn From_DeleteFromPlan_for_DeletePlanItem() {
        use super::DeleteFromPlan;

        let delete_from = DeleteFromPlan {
            table_name: TableName::new(None, "table".into()),
            alias: None,
            scan: ScanType::FullScan,
        };
        let delete_plan_item: DeletePlanItem = delete_from.clone().into();
        assert_eq!(delete_plan_item, DeletePlanItem::DeleteFrom(delete_from));
    }
}
