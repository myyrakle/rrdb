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
    // let foo = lib::utils::predule::get_system_env("RRDB_BASE_PATH");

    // println!("{}", foo);

    let text = r#"
    SELECT -2 * 5 AS foo
"#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    println!("{:?}", parser.parse().unwrap());

    Ok(())
}
