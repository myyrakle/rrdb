#![cfg(test)]

use crate::ast::dml::{OrderByNulls, SelectWildCard};
use crate::ast::predule::{
    BinaryOperator, BinaryOperatorExpression, GroupByItem, HavingClause, JoinClause, JoinType,
    OrderByItem, OrderByType, SQLExpression, SelectColumn, SelectItem, SelectQuery, TableName,
    WhereClause,
};
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

#[test]
pub fn select_from_1() {
    let text = r#"
        SELECT 1 as asdf
        FROM foo.bar
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(SQLExpression::Integer(1).into())
                .set_alias("asdf".into())
                .build(),
        )
        .set_from_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_from_2() {
    let text = r#"
        SELECT 1 as asdf
        FROM foo.bar as boom
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(SQLExpression::Integer(1).into())
                .set_alias("asdf".into())
                .build(),
        )
        .set_from_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .set_from_alias("boom".into())
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_from_3() {
    let text = r#"
        SELECT boom.number as number
        FROM (
            select 1 as number
            from foo.bar as temp
        ) as boom
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
                        .set_item(SQLExpression::Integer(1).into())
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_from_4() {
    let text = r#"
        SELECT *
        FROM foo.bar
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_wildcard(SelectWildCard { alias: None })
        .set_from_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_inner_join_1() {
    let text = r#"
        SELECT 
            p.content as post
            , c.content as `comment`
        FROM post as p
        INNER JOIN `comment` as c
        on p.id = c.post_id
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_inner_join_2() {
    let text = r#"
        SELECT 
            p.content as post
            , c.content as `comment`
        FROM post as p
        JOIN `comment` as c
        on p.id = c.post_id
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_left_join_1() {
    let text = r#"
        SELECT 
            p.content as post
            , c.content as `comment`
        FROM post as p
        LEFT JOIN `comment` as c
        on p.id = c.post_id
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_left_join_2() {
    let text = r#"
        SELECT 
            p.content as post
            , c.content as `comment`
        FROM post as p
        LEFT OUTER JOIN `comment` as c
        on p.id = c.post_id
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_right_join_1() {
    let text = r#"
        SELECT 
            p.content as post
            , c.content as `comment`
        FROM post as p
        RIGHT JOIN `comment` as c
        on p.id = c.post_id
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_right_join_2() {
    let text = r#"
        SELECT 
            p.content as post
            , c.content as `comment`
        FROM post as p
        RIGHT OUTER JOIN `comment` as c
        on p.id = c.post_id
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_full_join_1() {
    let text = r#"
        SELECT 
            p.content as post
            , c.content as `comment`
        FROM post as p
        FULL JOIN `comment` as c
        on p.id = c.post_id
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_full_join_2() {
    let text = r#"
        SELECT 
            p.content as post
            , c.content as `comment`
        FROM post as p
        FULL OUTER JOIN `comment` as c
        on p.id = c.post_id
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_where_1() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        where p.user_id = 1
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_order_by_1() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        ORDER BY p.user_id ASC
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_order_by_2() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        ORDER BY p.user_id ASC, p.id DESC
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_order_by_3() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        ORDER BY p.user_id NULLS FIRST
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_order_by_4() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        ORDER BY p.user_id NULLS LAST
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_group_by_1() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        GROUP BY p.content
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
            item: SelectColumn::new(Some("p".into()), "content".into()).into(),
        })
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_group_by_2() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        GROUP BY p.content, p.user_id
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
            item: SelectColumn::new(Some("p".into()), "content".into()).into(),
        })
        .add_group_by(GroupByItem {
            item: SelectColumn::new(Some("p".into()), "user_id".into()).into(),
        })
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn error_select_group_by_1() {
    let text = r#"
        SELECT 
            COUNT(p.a),
            p.b,
            p.c
        FROM post as p
        GROUP BY p.b
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    assert!(parser.parse(ParserContext::default()).is_err());
}

#[test]
pub fn select_group_by_having_1() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        GROUP BY p.content
        HAVING p.content = 'FOO'
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
            item: SelectColumn::new(Some("p".into()), "content".into()).into(),
        })
        .set_having(HavingClause {
            expression: BinaryOperatorExpression {
                operator: BinaryOperator::Eq,
                lhs: SelectColumn::new(Some("p".into()), "content".into()).into(),
                rhs: SQLExpression::String("FOO".into()),
            }
            .into(),
        })
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_offset_1() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        OFFSET 5
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_limit_1() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        LIMIT 5
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_offset_limit_1() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        OFFSET 5
        LIMIT 10
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_limit_offset_1() {
    let text = r#"
        SELECT 
            p.content as post
        FROM post as p
        LIMIT 10
        OFFSET 5
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn select_subquery_1() {
    let text = r#"
        SELECT 
            ff.number as number,
            (
                select 1 as number
                from foo.bar as temp
                limit 1
            ) as asdf
        FROM foo.foo as ff
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
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
                                .set_item(SQLExpression::Integer(1).into())
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}
