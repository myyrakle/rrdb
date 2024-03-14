#![cfg(test)]

use crate::ast::predule::{
    InsertQuery, InsertValue, SQLExpression, SelectColumn, SelectItem, SelectQuery, TableName,
};
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

#[test]
pub fn insert_into_values_1() {
    let text = r#"
        INSERT INTO foo.bar(a, b, c)
        Values(1, 2, 3)
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = InsertQuery::builder()
        .set_into_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .set_columns(vec!["a".into(), "b".into(), "c".into()])
        .set_values(vec![InsertValue {
            list: vec![
                Some(SQLExpression::Integer(1)),
                Some(SQLExpression::Integer(2)),
                Some(SQLExpression::Integer(3)),
            ],
        }])
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn insert_into_values_2() {
    let text = r#"
        INSERT INTO foo.bar(a, b, c)
        Values(1, 2, 3), (4, 5, 6)
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = InsertQuery::builder()
        .set_into_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .set_columns(vec!["a".into(), "b".into(), "c".into()])
        .set_values(vec![
            InsertValue {
                list: vec![
                    Some(SQLExpression::Integer(1)),
                    Some(SQLExpression::Integer(2)),
                    Some(SQLExpression::Integer(3)),
                ],
            },
            InsertValue {
                list: vec![
                    Some(SQLExpression::Integer(4)),
                    Some(SQLExpression::Integer(5)),
                    Some(SQLExpression::Integer(6)),
                ],
            },
        ])
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn insert_into_values_3() {
    let text = r#"
        INSERT INTO foo.bar(a, b, c)
        Values(1, 2, 3), (4, 5, DEFAULT)
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = InsertQuery::builder()
        .set_into_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .set_columns(vec!["a".into(), "b".into(), "c".into()])
        .set_values(vec![
            InsertValue {
                list: vec![
                    Some(SQLExpression::Integer(1)),
                    Some(SQLExpression::Integer(2)),
                    Some(SQLExpression::Integer(3)),
                ],
            },
            InsertValue {
                list: vec![
                    Some(SQLExpression::Integer(4)),
                    Some(SQLExpression::Integer(5)),
                    None,
                ],
            },
        ])
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn insert_into_select_1() {
    let text = r#"
        INSERT INTO foo.bar(a, b, c)
        Select s.a, s.b, s.c from boom.some as s
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = InsertQuery::builder()
        .set_into_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .set_columns(vec!["a".into(), "b".into(), "c".into()])
        .set_select(
            SelectQuery::builder()
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("s".into()), "a".into()).into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("s".into()), "b".into()).into())
                        .build(),
                )
                .add_select_item(
                    SelectItem::builder()
                        .set_item(SelectColumn::new(Some("s".into()), "c".into()).into())
                        .build(),
                )
                .set_from_table(TableName {
                    database_name: Some("boom".into()),
                    table_name: "some".into(),
                })
                .set_from_alias("s".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}
