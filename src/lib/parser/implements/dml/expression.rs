use std::error::Error;

use crate::lib::ast::predule::SQLExpression;
use crate::lib::errors::predule::ParsingError;
use crate::lib::lexer::predule::Token;
use crate::lib::parser::predule::Parser;

impl Parser {
    pub(crate) fn parse_expression(&mut self) -> Result<SQLExpression, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Operator(operator) => match operator {
                _ => {
                    return Err(ParsingError::boxed(format!(
                        "unexpected operator: {:?}",
                        operator
                    )));
                }
            },
            Token::Integer(integer) => {
                let lhs = integer;

                if self.next_token_is_binary_operator() {
                    // TODO: 2항 표현식 파싱 진입
                } else {
                    return Ok(SQLExpression::Integer(lhs));
                }
            }
            Token::Float(float) => {
                let lhs = float;

                if self.next_token_is_binary_operator() {
                    // TODO: 2항 표현식 파싱 진입
                } else {
                    return Ok(SQLExpression::Float(lhs));
                }
            }
            Token::Identifier(identifier) => {
                self.unget_next_token(Token::Identifier(identifier));

                let select_column = self.parse_select_column()?;

                if self.next_token_is_binary_operator() {
                    // TODO: 2항 표현식 파싱 진입
                } else {
                    return Ok(SQLExpression::SelectColumn(select_column));
                }
            }
            Token::String(string) => {
                let lhs = string;

                if self.next_token_is_binary_operator() {
                    // TODO: 2항 표현식 파싱 진입
                } else {
                    return Ok(SQLExpression::String(lhs));
                }
            }
            Token::Boolean(boolean) => {
                let lhs = boolean;

                if self.next_token_is_binary_operator() {
                    // TODO: 2항 표현식 파싱 진입
                } else {
                    return Ok(SQLExpression::Boolean(lhs));
                }
            }
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

    /**
     * 소괄호 파싱
    parenexpr ::= '(' expression ')'
    */
    pub(crate) fn parse_binary_expression(&mut self) -> Result<SQLExpression, Box<dyn Error>> {
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
    }
}
