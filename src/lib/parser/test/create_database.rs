#[cfg(test)]
use crate::lib::ast::ddl::CreateDatabaseQuery;
#[cfg(test)]
use crate::lib::parser::Parser;

#[test]
pub fn create_database() {
    let text = r#"
        CREATE DATABASE IF Not exists test_db;
    "#
    .to_owned();

    let mut parser = Parser::new(text);

    let expected = CreateDatabaseQuery::builder()
        .set_name("test_db".to_owned())
        .set_if_not_exists(true)
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}
