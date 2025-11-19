#![cfg(test)]

use crate::engine::ast::ddl::create_table::CreateTableQuery;
use crate::engine::ast::types::{Column, DataType, TableName};
use crate::engine::parser::context::ParserContext;
use crate::engine::parser::predule::Parser;

#[test]
pub fn create_table() {
    let text = r#"
        CREATE TABLE "test_db".person
        (
            id INTEGER PRIMARY KEY,
            name varchar(100),
            age INTEGER
        );
    "#
    .to_owned();

    let mut parser = Parser::with_string(text).unwrap();

    let expected = CreateTableQuery::builder()
        .set_table(TableName::new(
            Some("test_db".to_owned()),
            "person".to_owned(),
        ))
        .add_column(
            Column::builder()
                .set_name("id".to_owned())
                .set_data_type(DataType::Int)
                .set_primary_key(true)
                .build(),
        )
        .add_column(
            Column::builder()
                .set_name("name".to_owned())
                .set_data_type(DataType::Varchar(100))
                .build(),
        )
        .add_column(
            Column::builder()
                .set_name("age".to_owned())
                .set_data_type(DataType::Int)
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}
