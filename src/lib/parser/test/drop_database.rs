#[cfg(test)]
use crate::lib::ast::ddl::DropDatabaseQuery;
#[cfg(test)]
use crate::lib::parser::Parser;

#[test]
pub fn drop_database() {
    let text = r#"
        DROP DATABASE IF exists test_db;
    "#
    .to_owned();

    let mut parser = Parser::new(text);

    let expected = DropDatabaseQuery::builder()
        .set_name("test_db".to_owned())
        .set_if_exists(true)
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}
