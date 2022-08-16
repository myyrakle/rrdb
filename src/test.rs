#![allow(unused_imports)]
#![allow(dead_code)]

pub mod command;
pub mod lib;

use crate::lib::ast::predule::{
    BinaryOperator, BinaryOperatorExpression, SQLExpression, SelectItem, SelectQuery,
};
use crate::lib::parser::predule::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = r#"
    INSERT INTO foo.bar(a, b, c)
        Values(1, 2, 3)
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    parser.parse().unwrap();

    Ok(())
}
