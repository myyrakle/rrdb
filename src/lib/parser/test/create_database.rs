#[cfg(test)]
use crate::lib::ast::ddl::CreateDatabaseQuery;
#[cfg(test)]
use crate::lib::parser::predule::Parser;

#[test]
pub fn create_database_1() {
    let text = r#"
        CREATE DATABASE IF Not exists test_db;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = CreateDatabaseQuery::builder()
        .set_name("test_db".to_owned())
        .set_if_not_exists(true)
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}

#[test]
pub fn create_database_2() {
    let text = r#"
        CREATE DATABASE test_db;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = CreateDatabaseQuery::builder()
        .set_name("test_db".to_owned())
        .set_if_not_exists(false)
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}
