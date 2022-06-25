use std::error::Error;
use std::thread::current;

use crate::lib::lexer::Token;
use crate::lib::parser::Parser;
use crate::lib::{ParsingError, SQLExpression, SQLStatement, SelectQuery};

impl Parser {
    pub(crate) fn parse_expression(&mut self) -> Result<SQLExpression, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let query_builder = SelectQuery::builder();

        let current_token = self.get_next_token();

        match current_token {
            Token::Integer(integer)=> {

            }, 
            _ => {
                return Err(ParsingError::boxed(format!("unexpected token: {:?}", current_token)));
            }
        }

        return Err(ParsingError::boxed("need more tokens"));
    }
}
