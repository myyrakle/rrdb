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
        SELECT boom.number as number
        FROM (
            select 1 as number
            from foo.bar as temp
        ) as boom
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    parser.parse().unwrap();

    Ok(())
}
