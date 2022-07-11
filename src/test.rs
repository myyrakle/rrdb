pub mod command;
pub mod lib;

//use lib::lexer::Tokenizer;
use lib::parser::predule::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let foo = lib::utils::get_system_env("RRDB_BASE_PATH");

    println!("{}", foo);

    let text = r#"
       
        drop table if exists "foo_db".foo;
        "#
    .to_owned();

    let _foo = r#" CREATE DATABASE if not exists "test_db";
CREATE TABLE if not exists "test_db"."person"
(
    id INTEGER PRIMARY KEY,
    name varchar(100),
    age INTEGER
);
drop database "foo_db";"#;

    // let tokens = Tokenizer::string_to_tokens(text);
    // println!("{:?}", tokens);

    let mut parser = Parser::new(text);
    let result = parser.parse();
    println!("{:?}", result);

    let text = r#"
        drop table "bar"."person"; "#
        .to_owned();

    // let tokens = Tokenizer::string_to_tokens(text);
    // println!("{:?}", tokens);

    let mut parser = Parser::new(text);
    let result = parser.parse();
    println!("{:?}", result);

    Ok(())
}
