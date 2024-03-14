use crate::ast::predule::{
    DMLStatement, FromClause, FromTarget, GroupByClause, GroupByItem, HavingClause, JoinClause,
    OrderByClause, OrderByItem, SQLExpression, SQLStatement, SelectColumn, SelectItem,
    SubqueryExpression, TableName, WhereClause,
};

use serde::{Deserialize, Serialize};

use super::{SelectKind, SelectWildCard};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct SelectQuery {
    pub select_items: Vec<SelectKind>,
    pub has_aggregate: bool,
    pub from_table: Option<FromClause>,
    pub join_clause: Vec<JoinClause>,
    pub where_clause: Option<WhereClause>,
    pub order_by_clause: Option<OrderByClause>,
    pub group_by_clause: Option<GroupByClause>,
    pub having_clause: Option<HavingClause>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl SelectQuery {
    pub fn builder() -> Self {
        SelectQuery {
            select_items: vec![],
            from_table: None,
            join_clause: vec![],
            where_clause: None,
            group_by_clause: None,
            having_clause: None,
            order_by_clause: None,
            limit: None,
            offset: None,
            has_aggregate: false,
        }
    }

    pub fn add_select_item(mut self, item: SelectItem) -> Self {
        self.select_items.push(SelectKind::SelectItem(item));
        self
    }

    pub fn add_select_wildcard(mut self, item: SelectWildCard) -> Self {
        self.select_items.push(SelectKind::WildCard(item));
        self
    }

    pub fn set_from_table(mut self, from: TableName) -> Self {
        self.from_table = Some(from.into());
        self
    }

    pub fn has_from_table(&self) -> bool {
        self.from_table.is_some()
    }

    pub fn set_from_subquery(mut self, from: SubqueryExpression) -> Self {
        self.from_table = Some(FromClause {
            from: FromTarget::Subquery(from),
            alias: None,
        });
        self
    }

    pub fn set_from_alias(mut self, alias: String) -> Self {
        if self.from_table.is_some() {
            self.from_table = self.from_table.map(|mut e| {
                e.alias = Some(alias);
                e
            });
        }
        self
    }

    pub fn add_join(mut self, join: JoinClause) -> Self {
        self.join_clause.push(join);
        self
    }

    pub fn set_where(mut self, where_clause: WhereClause) -> Self {
        self.where_clause = Some(where_clause);
        self
    }

    pub fn add_order_by(mut self, item: OrderByItem) -> Self {
        match self.order_by_clause {
            Some(ref mut order_by_clause) => {
                order_by_clause.order_by_items.push(item);
            }
            None => {
                self.order_by_clause = Some(OrderByClause {
                    order_by_items: vec![item],
                })
            }
        }

        self
    }

    pub fn add_group_by(mut self, item: GroupByItem) -> Self {
        match self.group_by_clause {
            Some(ref mut group_by_clause) => {
                group_by_clause.group_by_items.push(item);
            }
            None => {
                self.group_by_clause = Some(GroupByClause {
                    group_by_items: vec![item],
                })
            }
        }

        self
    }

    pub fn has_group_by(&self) -> bool {
        match self.group_by_clause {
            Some(ref group_by_clause) => !group_by_clause.group_by_items.is_empty(),
            None => false,
        }
    }

    pub fn set_having(mut self, having_clause: HavingClause) -> Self {
        self.having_clause = Some(having_clause);
        self
    }

    pub fn set_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn set_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn set_has_aggregate(mut self, has_aggregate: bool) -> Self {
        self.has_aggregate = has_aggregate;
        self
    }

    pub fn has_aggregate(&self) -> bool {
        for item in &self.select_items {
            match item {
                SelectKind::SelectItem(item) => {
                    let item = item.item.as_ref().unwrap();

                    if item.has_aggregate() {
                        return true;
                    }
                }
                SelectKind::WildCard(_) => return false,
            }
        }

        false
    }

    pub fn get_non_aggregate_column(&self) -> Vec<SelectColumn> {
        let mut list = vec![];

        for item in &self.select_items {
            match item {
                SelectKind::SelectItem(item) => {
                    let item = item.item.as_ref().unwrap();
                    let mut none_aggregate_columns = item.find_non_aggregate_columns();
                    list.append(&mut none_aggregate_columns);
                }
                SelectKind::WildCard(_) => {}
            }
        }

        list
    }

    pub fn get_aggregate_column(&self) -> Vec<SelectColumn> {
        let mut list = vec![];

        for item in &self.select_items {
            match item {
                SelectKind::SelectItem(item) => {
                    let item = item.item.as_ref().unwrap();
                    let mut aggregate_columns = item.find_aggregate_columns();
                    list.append(&mut aggregate_columns);
                }
                SelectKind::WildCard(_) => {}
            }
        }

        list
    }

    pub fn build(self) -> SelectQuery {
        self
    }
}

impl From<SelectQuery> for SQLStatement {
    fn from(value: SelectQuery) -> SQLStatement {
        SQLStatement::DML(DMLStatement::SelectQuery(value))
    }
}

impl From<SelectQuery> for SubqueryExpression {
    fn from(value: SelectQuery) -> SubqueryExpression {
        SubqueryExpression::Select(Box::new(value))
    }
}

impl From<SelectQuery> for SQLExpression {
    fn from(value: SelectQuery) -> SQLExpression {
        SQLExpression::Subquery(SubqueryExpression::Select(Box::new(value)))
    }
}
