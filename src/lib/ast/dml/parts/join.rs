use crate::lib::ast::predule::{SQLExpression, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub left: TableName,
    pub right: TableName,
    pub on: Option<SQLExpression>,
}

impl JoinClause {}

#[derive(Clone, Debug, PartialEq)]
pub enum JoinType {
    InnerJoin,
    LeftOuterJoin,
    RightOuterJoin,
    FullOuterJoin,
}
