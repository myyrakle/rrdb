#![cfg(test)]

use crate::lib::ast::predule::{SQLExpression, TableName, UpdateItem, UpdateQuery};
use crate::lib::parser::predule::Parser;

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

    assert_eq!(parser.parse().unwrap(), vec![expected.into()],);
}
