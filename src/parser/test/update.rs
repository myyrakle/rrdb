#![cfg(test)]

use crate::ast::predule::{
    BinaryOperator, BinaryOperatorExpression, SQLExpression, SelectColumn, TableName, UpdateItem,
    UpdateQuery, WhereClause,
};
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

#[test]
pub fn update_set_1() {
    let text = r#"
        Update foo.bar
        set
            a = 1
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = UpdateQuery::builder()
        .set_target_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .add_update_item(UpdateItem {
            column: "a".into(),
            value: SQLExpression::Integer(1),
        })
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn update_set_2() {
    let text = r#"
        Update foo.bar
        set
            a = 1,
            b = 2
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = UpdateQuery::builder()
        .set_target_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .add_update_item(UpdateItem {
            column: "a".into(),
            value: SQLExpression::Integer(1),
        })
        .add_update_item(UpdateItem {
            column: "b".into(),
            value: SQLExpression::Integer(2),
        })
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn update_set_where_1() {
    let text = r#"
        Update foo.bar
        set
            a = 1
        where a = 5
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = UpdateQuery::builder()
        .set_target_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .add_update_item(UpdateItem {
            column: "a".into(),
            value: SQLExpression::Integer(1),
        })
        .set_where(WhereClause {
            expression: BinaryOperatorExpression {
                operator: BinaryOperator::Eq,
                lhs: SelectColumn::new(None, "a".into()).into(),
                rhs: SQLExpression::Integer(5),
            }
            .into(),
        })
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}
