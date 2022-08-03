#![cfg(test)]

use crate::lib::ast::predule::{SQLExpression, SelectColumn, SelectItem, SelectQuery, TableName};
use crate::lib::parser::predule::Parser;

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

    assert_eq!(parser.parse().unwrap(), vec![expected],);
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

    assert_eq!(parser.parse().unwrap(), vec![expected],);
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
                .build(),
        )
        .set_from_alias("boom".into())
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}
