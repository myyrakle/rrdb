#![cfg(test)]
use crate::lib::ast::ddl::{AlterDatabaseAction, AlterDatabaseQuery, AlterDatabaseRenameTo};
use crate::lib::parser::context::ParserContext;
use crate::lib::parser::predule::Parser;

#[test]
pub fn alter_table_rename_1() {
    let text = r#"
        ALTER DATABASE foo RENAME TO bar;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = AlterDatabaseQuery::builder()
        .set_name("foo".to_owned())
        .set_action(AlterDatabaseAction::RenameTo(AlterDatabaseRenameTo {
            name: "bar".into(),
        }))
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}
