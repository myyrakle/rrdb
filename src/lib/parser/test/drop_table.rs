#[cfg(test)]
use crate::lib::ast::ddl::DropTableQuery;
#[cfg(test)]
use crate::lib::ast::types::TableName;
use crate::lib::parser::context::ParserContext;
#[cfg(test)]
use crate::lib::parser::predule::Parser;

#[test]
pub fn drop_table() {
    let text = r#"
        drop table if exists "foo_db".foo;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = DropTableQuery::builder()
        .set_table(TableName::new(Some("foo_db".to_owned()), "foo".to_owned()))
        .set_if_exists(true)
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}
