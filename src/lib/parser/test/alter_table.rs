#![cfg(test)]
use crate::lib::ast::ddl::{
    AlterDatabaseAction, AlterDatabaseQuery, AlterDatabaseRenameTo, AlterTableQuery,
    AlterTableRenameTo,
};
use crate::lib::ast::predule::TableName;
use crate::lib::parser::context::ParserContext;
use crate::lib::parser::predule::Parser;

#[test]
pub fn alter_table_rename_1() {
    let text = r#"
        ALTER TABLE foo RENAME TO bar;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = AlterTableQuery::builder()
        .set_table(TableName {
            table_name: "foo".to_owned(),
            database_name: None,
        })
        .set_action(AlterTableRenameTo { name: "bar".into() }.into())
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}
