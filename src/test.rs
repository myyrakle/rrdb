pub mod command;
pub mod lib;

use lib::lexer::Tokenizer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = r#"SELECT 1"#.to_owned();

    let mut tokenizer = Tokenizer::new(text);

    let mut tokens = vec![];

    while !tokenizer.is_eof() {
        tokens.push(tokenizer.get_token());
    }

    println!("{:?}", tokens);

    Ok(())
}
