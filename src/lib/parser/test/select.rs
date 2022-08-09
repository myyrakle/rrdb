#![cfg(test)]

use crate::lib::ast::predule::{
    BinaryOperator, BinaryOperatorExpression, JoinClause, JoinType, SQLExpression, SelectColumn,
    SelectItem, SelectQuery, TableName,
};
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

#[test]
pub fn select_inner_join_1() {
    let text = r#"
        SELECT 
            p.content as post
            , c.content as comment
        FROM post as p
        INNER JOIN comment as c
        on p.id = c.post_id
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

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}
