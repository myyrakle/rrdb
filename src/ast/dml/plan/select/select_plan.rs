use crate::ast::dml::parts::{group_by::GroupByClause, order_by::OrderByClause};

use super::{
    filter::FilterPlan, from::SelectFromPlan, join::JoinPlan, limit_offset::LimitOffsetPlan,
    subquery::SelectSubqueryPlan,
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

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::ast::{
        dml::{
            parts::join::JoinType,
            plan::select::{from::SelectFromPlan, join::JoinScanType, scan::ScanType},
        },
        types::{SQLExpression, TableName},
    };

    use super::*;

    #[test]
    fn From_SelectFromPlan_for_SelectPlanItem() {
        let select_from = SelectFromPlan {
            table_name: TableName::new(None, "table".into()),
            alias: None,
            scan: ScanType::FullScan,
        };
        let select_plan_item: SelectPlanItem = select_from.clone().into();
        assert_eq!(select_plan_item, SelectPlanItem::From(select_from));
    }

    #[test]
    fn From_SelectSubqueryPlan_for_SelectPlanItem() {
        let select_subquery = SelectSubqueryPlan {
            table_name: TableName::new(None, "table".into()),
            select_columns: vec![],
            index: None,
            filter: None,
        };
        let select_plan_item: SelectPlanItem = select_subquery.clone().into();
        assert_eq!(select_plan_item, SelectPlanItem::Subquery(select_subquery));
    }

    #[test]
    fn From_JoinPlan_for_SelectPlanItem() {
        let join = JoinPlan {
            join_type: JoinType::InnerJoin,
            left: TableName::new(None, "l".into()),
            right: TableName::new(None, "r".into()),
            join_scan_type: JoinScanType::Hash,
            select_columns: vec![],
            index: None,
            filter: None,
        };
        let select_plan_item: SelectPlanItem = join.clone().into();
        assert_eq!(select_plan_item, SelectPlanItem::Join(join));
    }

    #[test]
    fn From_OrderByClause_for_SelectPlanItem() {
        let order_by = OrderByClause {
            order_by_items: vec![],
        };
        let select_plan_item: SelectPlanItem = order_by.clone().into();
        assert_eq!(select_plan_item, SelectPlanItem::Order(order_by));
    }

    #[test]
    fn From_GroupByClause_for_SelectPlanItem() {
        let group_by = GroupByClause {
            group_by_items: vec![],
        };
        let select_plan_item: SelectPlanItem = group_by.clone().into();
        assert_eq!(select_plan_item, SelectPlanItem::Group(group_by));
    }

    #[test]
    fn From_LimitOffsetPlan_for_SelectPlanItem() {
        let limit_offset = LimitOffsetPlan {
            limit: None,
            offset: None,
        };
        let select_plan_item: SelectPlanItem = limit_offset.clone().into();
        assert_eq!(select_plan_item, SelectPlanItem::LimitOffset(limit_offset));
    }

    #[test]
    fn From_FilterPlan_for_SelectPlanItem() {
        let filter = FilterPlan {
            expression: SQLExpression::String("a".into()),
        };
        let select_plan_item: SelectPlanItem = filter.clone().into();
        assert_eq!(select_plan_item, SelectPlanItem::Filter(filter));
    }
}
