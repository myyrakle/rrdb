#![cfg(test)]

use crate::lib::ast::predule::{DeleteQuery, TableName};
use crate::lib::parser::predule::Parser;

#[test]
pub fn delete_from_1() {
    let text = r#"
        DELETE FROM foo.bar
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = DeleteQuery::builder()
        .set_from_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected.into()],);
}
