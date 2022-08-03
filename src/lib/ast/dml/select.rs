use crate::lib::ast::predule::{
    DMLStatement, FromClause, GroupByClause, OrderByClause, SQLStatement, TableName, WhereClause,
};

use super::SelectItem;

#[derive(Clone, Debug, PartialEq)]
pub struct SelectQuery {
    pub select_items: Vec<SelectItem>,
    pub from_table: Option<FromClause>, // 차후 서브쿼리 사용 가능하게 확장할 필요
    pub where_clause: Option<WhereClause>,
    pub group_by_clause: Option<GroupByClause>,
    pub order_by_clause: Option<OrderByClause>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl SelectQuery {
    pub fn builder() -> Self {
        SelectQuery {
            select_items: vec![],
            from_table: None,
            where_clause: None,
            group_by_clause: None,
            order_by_clause: None,
            limit: None,
            offset: None,
        }
    }

    pub fn add_select_item(mut self, item: SelectItem) -> Self {
        self.select_items.push(item);
        self
    }

    pub fn set_from_table(mut self, from: TableName) -> Self {
        self.from_table = Some(from.into());
        self
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DML(DMLStatement::SelectQuery(self))
    }
}

impl Into<FromClause> for SelectQuery {
    fn into(self) -> FromClause {
        FromClause::Subquery(Box::new(self))
    }
}
