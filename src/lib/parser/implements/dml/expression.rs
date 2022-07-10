use std::error::Error;
use std::thread::current;

use crate::lib::ast::predule::{identifier, ParsingError, SQLExpression};
use crate::lib::lexer::predule::Token;
use crate::lib::parser::Parser;

impl Parser {
    pub(crate) fn parse_expression(&mut self) -> Result<SQLExpression, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Operator(operator) => match operator {
                _ => {}
            },
            Token::Integer(integer) => {}
            Token::Float(float) => {}
            Token::Identifier(identifier) => {}
            Token::String(string) => {}
            Token::Boolean(boolean) => {}
            Token::LeftParentheses => {
                self.unget_next_token(current_token);
                return self.parse_parentheses_expression();
            }
            Token::RightParentheses => {
                // self.unget_next_token(current_token);
                return Err(ParsingError::boxed(format!(
                    "unexpected token: {:?}",
                    current_token
                )));
            }
            Token::As => {}
            Token::Comma => {}
            _ => {
                return Err(ParsingError::boxed(format!(
                    "unexpected token: {:?}",
                    current_token
                )));
            }
        }

        return Err(ParsingError::boxed("need more tokens"));
    }

    /**
     * 소괄호 파싱
    parenexpr ::= '(' expression ')'
    */
    pub(crate) fn parse_parentheses_expression(&mut self) -> Result<SQLExpression, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // ( 삼킴
        let current_token = self.get_next_token();

        if current_token != Token::RightParentheses {
            return Err(ParsingError::boxed(format!(
                "expected right parentheses. but your input is {:?}",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // 표현식 파싱
        let expression = self.parse_expression();

        // ) 삼킴
        let current_token = self.get_next_token();

        if current_token != Token::LeftParentheses {
            return Err(ParsingError::boxed(format!(
                "expected left parentheses. but your input is {:?}",
                current_token
            )));
        }

        expression
    }
}
