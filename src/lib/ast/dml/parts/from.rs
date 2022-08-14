use crate::lib::ast::predule::{SubqueryExpression, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct FromClause {
    pub from: FromTarget,
    pub alias: Option<String>,
}

impl FromClause {}

#[derive(Clone, Debug, PartialEq)]
pub enum FromTarget {
    Table(TableName),             // 일반 테이블 참조
    Subquery(SubqueryExpression), // 서브쿼리 참조
}

impl FromTarget {}
