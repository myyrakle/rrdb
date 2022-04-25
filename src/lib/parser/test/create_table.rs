#[cfg(test)]
use crate::lib::ast::ddl::CreateTableQuery;
#[cfg(test)]
use crate::lib::ast::types::{Column, DataType, TableName};
#[cfg(test)]
use crate::lib::parser::Parser;

#[test]
pub fn create_table() {
    let text = r#"
        CREATE TABLE person
        (
            id INTEGER PRIMARY KEY,
            name varchar(100),
            age INTEGER
        );
    "#
    .to_owned();

    let mut parser = Parser::new(text);

    let expected = CreateTableQuery::builder()
        .set_table(TableName::new(None, "person".to_owned()))
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

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}
