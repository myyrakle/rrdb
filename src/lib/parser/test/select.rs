#![cfg(test)]

use crate::lib::ast::predule::{SQLExpression, SelectItem, SelectQuery, TableName};
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


