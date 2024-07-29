#![cfg(test)]

use crate::ast::dml::expressions::binary::BinaryOperatorExpression;
use crate::ast::dml::expressions::call::CallExpression;
use crate::ast::dml::expressions::operators::BinaryOperator;
use crate::ast::dml::parts::_where::WhereClause;
use crate::ast::dml::parts::group_by::GroupByItem;
use crate::ast::dml::parts::having::HavingClause;
use crate::ast::dml::parts::join::{JoinClause, JoinType};
use crate::ast::dml::parts::order_by::{OrderByItem, OrderByNulls, OrderByType};
use crate::ast::dml::parts::select_item::{SelectItem, SelectWildCard};
use crate::ast::dml::select::SelectQuery;
use crate::ast::types::{
    AggregateFunction, BuiltInFunction, Function, SQLExpression, SelectColumn, TableName,
};
use crate::lexer::predule::OperatorToken;
use crate::lexer::tokens::Token;
use crate::parser::predule::Parser;

#[test]
fn test_select_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SelectQuery,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "성공: SELECT 1 as asdf FROM foo.bar".into(),
            input: vec![
                Token::Select,
                Token::Integer(1),
                Token::As,
                Token::Identifier("asdf".into()),
                Token::From,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SQLExpression::Integer(1))
                        .set_alias("asdf".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: "SELECT 1 as asdf FROM foo.bar as boom".into(),
            input: vec![
                Token::Select,
                Token::Integer(1),
                Token::As,
                Token::Identifier("asdf".into()),
                Token::From,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::As,
                Token::Identifier("boom".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SQLExpression::Integer(1))
                        .set_alias("asdf".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .set_from_alias("boom".into())
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT boom.number as number
                FROM (
                    select 1 as number
                    from foo.bar as temp
                ) as boom
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("boom".into()),
                Token::Period,
                Token::Identifier("number".into()),
                Token::As,
                Token::Identifier("number".into()),
                Token::From,
                Token::LeftParentheses,
                Token::Select,
                Token::Integer(1),
                Token::As,
                Token::Identifier("number".into()),
                Token::From,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::As,
                Token::Identifier("temp".into()),
                Token::RightParentheses,
                Token::As,
                Token::Identifier("boom".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("boom".into()), "number".into()).into())
                        .set_alias("number".into())
                        .build(),
                )
                .set_from_subquery(
                    SelectQuery::builder()
                        .add_select_item(
                            SelectItem::builder()
                                .set_item(SQLExpression::Integer(1))
                                .set_alias("number".into())
                                .build(),
                        )
                        .set_from_table(TableName {
                            database_name: Some("foo".into()),
                            table_name: "bar".into(),
                        })
                        .set_from_alias("temp".into())
                        .build()
                        .into(),
                )
                .set_from_alias("boom".into())
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT *
                FROM foo.bar
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Operator(OperatorToken::Asterisk),
                Token::From,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_wildcard(SelectWildCard { alias: None })
                .set_from_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                    , c.content as `comment`
                FROM post as p
                INNER JOIN `comment` as c
                on p.id = c.post_id
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("comment".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Inner,
                Token::Join,
                Token::Identifier("comment".into()),
                Token::As,
                Token::Identifier("c".into()),
                Token::On,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("post_id".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("c".into()), "content".into()).into())
                        .set_alias("comment".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_join(JoinClause {
                    join_type: JoinType::InnerJoin,
                    right: TableName::new(None, "comment".into()),
                    right_alias: Some("c".into()),
                    on: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                        rhs: SelectColumn::new(Some("c".into()), "post_id".into()).into(),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                    , c.content as `comment`
                FROM post as p
                JOIN `comment` as c
                on p.id = c.post_id
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("comment".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Join,
                Token::Identifier("comment".into()),
                Token::As,
                Token::Identifier("c".into()),
                Token::On,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("post_id".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("c".into()), "content".into()).into())
                        .set_alias("comment".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_join(JoinClause {
                    join_type: JoinType::InnerJoin,
                    right: TableName::new(None, "comment".into()),
                    right_alias: Some("c".into()),
                    on: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                        rhs: SelectColumn::new(Some("c".into()), "post_id".into()).into(),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                    , c.content as `comment`
                FROM post as p
                LEFT JOIN `comment` as c
                on p.id = c.post_id
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("comment".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Left,
                Token::Join,
                Token::Identifier("comment".into()),
                Token::As,
                Token::Identifier("c".into()),
                Token::On,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("post_id".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("c".into()), "content".into()).into())
                        .set_alias("comment".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_join(JoinClause {
                    join_type: JoinType::LeftOuterJoin,
                    right: TableName::new(None, "comment".into()),
                    right_alias: Some("c".into()),
                    on: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                        rhs: SelectColumn::new(Some("c".into()), "post_id".into()).into(),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                    , c.content as `comment`
                FROM post as p
                LEFT OUTER JOIN `comment` as c
                on p.id = c.post_id
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("comment".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Left,
                Token::Outer,
                Token::Join,
                Token::Identifier("comment".into()),
                Token::As,
                Token::Identifier("c".into()),
                Token::On,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("post_id".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("c".into()), "content".into()).into())
                        .set_alias("comment".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_join(JoinClause {
                    join_type: JoinType::LeftOuterJoin,
                    right: TableName::new(None, "comment".into()),
                    right_alias: Some("c".into()),
                    on: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                        rhs: SelectColumn::new(Some("c".into()), "post_id".into()).into(),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                    , c.content as `comment`
                FROM post as p
                RIGHT JOIN `comment` as c
                on p.id = c.post_id
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("comment".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Right,
                Token::Join,
                Token::Identifier("comment".into()),
                Token::As,
                Token::Identifier("c".into()),
                Token::On,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("post_id".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("c".into()), "content".into()).into())
                        .set_alias("comment".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_join(JoinClause {
                    join_type: JoinType::RightOuterJoin,
                    right: TableName::new(None, "comment".into()),
                    right_alias: Some("c".into()),
                    on: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                        rhs: SelectColumn::new(Some("c".into()), "post_id".into()).into(),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                    , c.content as `comment`
                FROM post as p
                RIGHT OUTER JOIN `comment` as c
                on p.id = c.post_id
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("comment".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Right,
                Token::Outer,
                Token::Join,
                Token::Identifier("comment".into()),
                Token::As,
                Token::Identifier("c".into()),
                Token::On,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("post_id".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("c".into()), "content".into()).into())
                        .set_alias("comment".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_join(JoinClause {
                    join_type: JoinType::RightOuterJoin,
                    right: TableName::new(None, "comment".into()),
                    right_alias: Some("c".into()),
                    on: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                        rhs: SelectColumn::new(Some("c".into()), "post_id".into()).into(),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                    , c.content as `comment`
                FROM post as p
                FULL JOIN `comment` as c
                on p.id = c.post_id
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("comment".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Full,
                Token::Join,
                Token::Identifier("comment".into()),
                Token::As,
                Token::Identifier("c".into()),
                Token::On,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("post_id".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("c".into()), "content".into()).into())
                        .set_alias("comment".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_join(JoinClause {
                    join_type: JoinType::FullOuterJoin,
                    right: TableName::new(None, "comment".into()),
                    right_alias: Some("c".into()),
                    on: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                        rhs: SelectColumn::new(Some("c".into()), "post_id".into()).into(),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                    , c.content as `comment`
                FROM post as p
                FULL OUTER JOIN `comment` as c
                on p.id = c.post_id
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("comment".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Full,
                Token::Outer,
                Token::Join,
                Token::Identifier("comment".into()),
                Token::As,
                Token::Identifier("c".into()),
                Token::On,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Identifier("c".into()),
                Token::Period,
                Token::Identifier("post_id".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("c".into()), "content".into()).into())
                        .set_alias("comment".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_join(JoinClause {
                    join_type: JoinType::FullOuterJoin,
                    right: TableName::new(None, "comment".into()),
                    right_alias: Some("c".into()),
                    on: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                        rhs: SelectColumn::new(Some("c".into()), "post_id".into()).into(),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                where p.user_id = 1
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Where,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("user_id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Integer(1),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .set_where(WhereClause {
                    expression: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(Some("p".into()), "user_id".into()).into(),
                        rhs: SQLExpression::Integer(1),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                ORDER BY p.user_id ASC;
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Order,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("user_id".into()),
                Token::Asc,
                Token::SemiColon,
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_order_by(OrderByItem {
                    item: SelectColumn::new(Some("p".into()), "user_id".into()).into(),
                    order_type: OrderByType::Asc,
                    nulls: OrderByNulls::First,
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                ORDER BY p.user_id ASC
                LIMIT 10;
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Order,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("user_id".into()),
                Token::Asc,
                Token::Limit,
                Token::Integer(10),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_order_by(OrderByItem {
                    item: SelectColumn::new(Some("p".into()), "user_id".into()).into(),
                    order_type: OrderByType::Asc,
                    nulls: OrderByNulls::First,
                })
                .set_limit(10)
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                ORDER BY SELECT ASC;
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Order,
                Token::By,
                Token::Select,
                Token::Asc,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                ORDER BY p.user_id ASC, p.id DESC
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Order,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("user_id".into()),
                Token::Asc,
                Token::Comma,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Desc,
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_order_by(OrderByItem {
                    item: SelectColumn::new(Some("p".into()), "user_id".into()).into(),
                    order_type: OrderByType::Asc,
                    nulls: OrderByNulls::First,
                })
                .add_order_by(OrderByItem {
                    item: SelectColumn::new(Some("p".into()), "id".into()).into(),
                    order_type: OrderByType::Desc,
                    nulls: OrderByNulls::First,
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                ORDER BY p.user_id NULLS FIRST
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Order,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("user_id".into()),
                Token::Nulls,
                Token::First,
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_order_by(OrderByItem {
                    item: SelectColumn::new(Some("p".into()), "user_id".into()).into(),
                    order_type: OrderByType::Asc,
                    nulls: OrderByNulls::First,
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                ORDER BY p.user_id NULLS LAST
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Order,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("user_id".into()),
                Token::Nulls,
                Token::Last,
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_order_by(OrderByItem {
                    item: SelectColumn::new(Some("p".into()), "user_id".into()).into(),
                    order_type: OrderByType::Asc,
                    nulls: OrderByNulls::Last,
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                GROUP BY p.content
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Group,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_group_by(GroupByItem {
                    item: SelectColumn::new(Some("p".into()), "content".into()),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                GROUP BY p.content;
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Group,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::SemiColon,
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_group_by(GroupByItem {
                    item: SelectColumn::new(Some("p".into()), "content".into()),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                GROUP BY SELECT;
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Group,
                Token::By,
                Token::Select,
                Token::SemiColon,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                GROUP BY p.content
                LIMIT 5;
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Group,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::Limit,
                Token::Integer(5),
                Token::SemiColon,
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_group_by(GroupByItem {
                    item: SelectColumn::new(Some("p".into()), "content".into()),
                })
                .set_limit(5)
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                GROUP BY p.content, p.user_id
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Group,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::Comma,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("user_id".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_group_by(GroupByItem {
                    item: SelectColumn::new(Some("p".into()), "content".into()),
                })
                .add_group_by(GroupByItem {
                    item: SelectColumn::new(Some("p".into()), "user_id".into()),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                실패: 
                SELECT 
                    COUNT(p.a),
                    p.b,
                    p.c
                FROM post as p
                GROUP BY p.b
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("count".into()),
                Token::LeftParentheses,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("a".into()),
                Token::RightParentheses,
                Token::Comma,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("b".into()),
                Token::Comma,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("c".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Group,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("b".into()),
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: r#"
                SELECT 
                    COUNT(p.a)
                FROM post as p
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("count".into()),
                Token::LeftParentheses,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("a".into()),
                Token::RightParentheses,
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SQLExpression::FunctionCall(CallExpression {
                            function: Function::BuiltIn(BuiltInFunction::Aggregate(
                                AggregateFunction::Count,
                            )),
                            arguments: vec![SelectColumn::new(Some("p".into()), "a".into()).into()],
                        }))
                        .build(),
                )
                .set_has_aggregate(true)
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                OFFSET 5
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Offset,
                Token::Integer(5),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .set_offset(5)
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                LIMIT 5
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Limit,
                Token::Integer(5),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .set_limit(5)
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                OFFSET 5
                LIMIT 10
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Offset,
                Token::Integer(5),
                Token::Limit,
                Token::Integer(10),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .set_offset(5)
                .set_limit(10)
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                LIMIT 10
                OFFSET 5
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Limit,
                Token::Integer(10),
                Token::Offset,
                Token::Integer(5),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .set_offset(5)
                .set_limit(10)
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                SELECT 
                    ff.number as number,
                    (
                        select 1 as number
                        from foo.bar as temp
                        limit 1
                    ) as asdf
                FROM foo.foo as ff
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("ff".into()),
                Token::Period,
                Token::Identifier("number".into()),
                Token::As,
                Token::Identifier("number".into()),
                Token::Comma,
                Token::LeftParentheses,
                Token::Select,
                Token::Integer(1),
                Token::As,
                Token::Identifier("number".into()),
                Token::From,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::As,
                Token::Identifier("temp".into()),
                Token::Limit,
                Token::Integer(1),
                Token::RightParentheses,
                Token::As,
                Token::Identifier("asdf".into()),
                Token::From,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("foo".into()),
                Token::As,
                Token::Identifier("ff".into()),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("ff".into()), "number".into()).into())
                        .set_alias("number".into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(
                            SelectQuery::builder()
                                .add_select_item(
                                    SelectItem::builder()
                                        .set_item(SQLExpression::Integer(1))
                                        .set_alias("number".into())
                                        .build(),
                                )
                                .set_from_table(TableName {
                                    database_name: Some("foo".into()),
                                    table_name: "bar".into(),
                                })
                                .set_from_alias("temp".into())
                                .set_limit(1)
                                .build()
                                .into(),
                        )
                        .set_alias("asdf".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "foo".into(),
                })
                .set_from_alias("ff".into())
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"실패: 빈 토큰"#.into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: r#"실패: SELECT"#.into(),
            input: vec![Token::Select],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: r#"실패: UPDATE"#.into(),
            input: vec![Token::Update],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: r#"SELECT 1 WHERE"#.into(),
            input: vec![Token::Select, Token::Integer(1), Token::Where],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: r#"SELECT 1;"#.into(),
            input: vec![Token::Select, Token::Integer(1), Token::SemiColon],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SQLExpression::Integer(1))
                        .build(),
                )
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                집계함수가 사용된 컬럼이 group by에 있다면 오류

                SELECT 
                    COUNT(p.a)
                FROM post as p 
                GROUP BY p.a
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("count".into()),
                Token::LeftParentheses,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("a".into()),
                Token::RightParentheses,
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Group,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("a".into()),
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: r#"
                SELECT 
                    p.content as post
                FROM post as p
                GROUP BY p.content, p.user_id
                HAVING p.user_id > 1
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Group,
                Token::By,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::Comma,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("user_id".into()),
                Token::Having,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("user_id".into()),
                Token::Operator(OperatorToken::Gt),
                Token::Integer(1),
            ],
            expected: SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("p".into()), "content".into()).into())
                        .set_alias("post".into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: None,
                    table_name: "post".into(),
                })
                .set_from_alias("p".into())
                .add_group_by(GroupByItem {
                    item: SelectColumn::new(Some("p".into()), "content".into()),
                })
                .add_group_by(GroupByItem {
                    item: SelectColumn::new(Some("p".into()), "user_id".into()),
                })
                .set_having(HavingClause {
                    expression: BinaryOperatorExpression {
                        operator: BinaryOperator::Gt,
                        lhs: SelectColumn::new(Some("p".into()), "user_id".into()).into(),
                        rhs: SQLExpression::Integer(1),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: r#"
                실패: GROUP BY 없는 HAVING 절
                SELECT 
                    p.content as post
                FROM post as p
                HAVING p.user_id > 1
            "#
            .into(),
            input: vec![
                Token::Select,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("content".into()),
                Token::As,
                Token::Identifier("post".into()),
                Token::From,
                Token::Identifier("post".into()),
                Token::As,
                Token::Identifier("p".into()),
                Token::Having,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("user_id".into()),
                Token::Operator(OperatorToken::Gt),
                Token::Integer(1),
            ],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.handle_select_query(Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected.into(), "TC: {}", t.name);
        }
    }
}

#[test]
fn test_parse_select_item() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SelectItem,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "1".into(),
            input: vec![Token::Integer(1)],
            expected: SelectItem::builder()
                .set_item(SQLExpression::Integer(1))
                .build(),
            want_error: false,
        },
        TestCase {
            name: "1 as one".into(),
            input: vec![
                Token::Integer(1),
                Token::As,
                Token::Identifier("one".into()),
            ],
            expected: SelectItem::builder()
                .set_item(SQLExpression::Integer(1))
                .set_alias("one".into())
                .build(),
            want_error: false,
        },
        TestCase {
            name: "실패: 1 as".into(),
            input: vec![Token::Integer(1), Token::As],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: 1 as SELECT".into(),
            input: vec![Token::Integer(1), Token::As, Token::Select],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_select_item(Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected, "TC: {}", t.name);
        }
    }
}

#[test]
fn test_parse_order_by_item() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: OrderByItem,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "p.id ASC".into(),
            input: vec![
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Asc,
            ],
            expected: OrderByItem {
                item: SelectColumn::new(Some("p".into()), "id".into()).into(),
                order_type: OrderByType::Asc,
                nulls: OrderByNulls::First,
            },
            want_error: false,
        },
        TestCase {
            name: "p.id".into(),
            input: vec![
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
            ],
            expected: OrderByItem {
                item: SelectColumn::new(Some("p".into()), "id".into()).into(),
                order_type: OrderByType::Asc,
                nulls: OrderByNulls::First,
            },
            want_error: false,
        },
        TestCase {
            name: "p.id DESC".into(),
            input: vec![
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Desc,
            ],
            expected: OrderByItem {
                item: SelectColumn::new(Some("p".into()), "id".into()).into(),
                order_type: OrderByType::Desc,
                nulls: OrderByNulls::First,
            },
            want_error: false,
        },
        TestCase {
            name: "p.id NULLS FIRST".into(),
            input: vec![
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Nulls,
                Token::First,
            ],
            expected: OrderByItem {
                item: SelectColumn::new(Some("p".into()), "id".into()).into(),
                order_type: OrderByType::Asc,
                nulls: OrderByNulls::First,
            },
            want_error: false,
        },
        TestCase {
            name: "p.id NULLS LAST".into(),
            input: vec![
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Nulls,
                Token::Last,
            ],
            expected: OrderByItem {
                item: SelectColumn::new(Some("p".into()), "id".into()).into(),
                order_type: OrderByType::Asc,
                nulls: OrderByNulls::Last,
            },
            want_error: false,
        },
        TestCase {
            name: "p.id NULLS SELECT".into(),
            input: vec![
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Nulls,
                Token::Select,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: p.id NULLS".into(),
            input: vec![
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Nulls,
            ],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_order_by_item(Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected, "TC: {}", t.name);
        }
    }
}

#[test]
fn test_parse_group_by_item() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: GroupByItem,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "p.id".into(),
            input: vec![
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
            ],
            expected: GroupByItem {
                item: SelectColumn::new(Some("p".into()), "id".into()),
            },
            want_error: false,
        },
        TestCase {
            name: "실패: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_group_by_item(Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected, "TC: {}", t.name);
        }
    }
}

#[test]
fn test_parse_join() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: JoinClause,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "foo.bar as fb ON p.id = fb.id".into(),
            input: vec![
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::As,
                Token::Identifier("fb".into()),
                Token::On,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Identifier("fb".into()),
                Token::Period,
                Token::Identifier("id".into()),
            ],
            expected: JoinClause {
                join_type: JoinType::InnerJoin,
                right: TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                },
                right_alias: Some("fb".into()),
                on: BinaryOperatorExpression {
                    operator: BinaryOperator::Eq,
                    lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                    rhs: SelectColumn::new(Some("fb".into()), "id".into()).into(),
                }
                .into(),
            },
            want_error: false,
        },
        TestCase {
            name: "foo.bar ON p.id = fb.id".into(),
            input: vec![
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::On,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Identifier("fb".into()),
                Token::Period,
                Token::Identifier("id".into()),
            ],
            expected: JoinClause {
                join_type: JoinType::InnerJoin,
                right: TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                },
                right_alias: None,
                on: BinaryOperatorExpression {
                    operator: BinaryOperator::Eq,
                    lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                    rhs: SelectColumn::new(Some("fb".into()), "id".into()).into(),
                }
                .into(),
            },
            want_error: false,
        },
        TestCase {
            name: "foo.bar;".into(),
            input: vec![
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::SemiColon,
            ],
            expected: JoinClause {
                join_type: JoinType::InnerJoin,
                right: TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                },
                right_alias: None,
                on: None,
            },
            want_error: false,
        },
        TestCase {
            name: "foo.bar".into(),
            input: vec![
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
            ],
            expected: JoinClause {
                join_type: JoinType::InnerJoin,
                right: TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                },
                right_alias: None,
                on: None,
            },
            want_error: false,
        },
        TestCase {
            name: "실패: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_join(JoinType::InnerJoin, Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected, "TC: {}", t.name);
        }
    }
}

#[test]
fn test_parse_where() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: WhereClause,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "WHERE p.id = 1".into(),
            input: vec![
                Token::Where,
                Token::Identifier("p".into()),
                Token::Period,
                Token::Identifier("id".into()),
                Token::Operator(OperatorToken::Eq),
                Token::Integer(1),
            ],
            expected: WhereClause {
                expression: BinaryOperatorExpression {
                    operator: BinaryOperator::Eq,
                    lhs: SelectColumn::new(Some("p".into()), "id".into()).into(),
                    rhs: SQLExpression::Integer(1),
                }
                .into(),
            },
            want_error: false,
        },
        TestCase {
            name: "실패: SELECT".into(),
            input: vec![Token::Select],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_where(Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected, "TC: {}", t.name);
        }
    }
}
