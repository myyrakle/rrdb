use crate::lib::ast::predule::{SQLStatement, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct FromClause {
    pub from: FromTarget,
    pub alias: Option<String>,
}

impl FromClause {}

#[derive(Clone, Debug, PartialEq)]
pub enum FromTarget {
    Table(TableName),            // 일반 테이블 참조
    Subquery(Box<SQLStatement>), // 서브쿼리 참조
}

impl FromTarget {}
