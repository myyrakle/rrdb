pub mod command;
pub mod lib;

//use lib::lexer::Tokenizer;
use lib::parser::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = r#"
        CREATE TABLE if not exists "bar"."person"
        (
            id INTEGER PRIMARY KEY,
            name varchar(100),
            age INTEGER
        ); "#
        .to_owned();

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
