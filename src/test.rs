#![allow(unused_imports)]
#![allow(dead_code)]

pub mod command;
pub mod lib;

use lib::parser::context::ParserContext;

use crate::lib::ast::predule::{
    BinaryOperator, BinaryOperatorExpression, SQLExpression, SelectItem, SelectQuery,
};
use crate::lib::parser::predule::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = r#"
    create database asdf;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    parser.parse(ParserContext::default()).unwrap();

    Ok(())
}
