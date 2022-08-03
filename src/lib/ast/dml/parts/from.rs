use crate::lib::ast::predule::{SelectQuery, TableName};

#[derive(Clone, Debug, PartialEq)]
pub enum FromClause {
    Table(TableName),           // 일반 테이블 참조
    Subquery(Box<SelectQuery>), // 서브쿼리 참조
}

impl FromClause {}
