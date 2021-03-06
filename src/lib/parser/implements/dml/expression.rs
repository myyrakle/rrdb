use std::convert::TryInto;
use std::error::Error;

use crate::lib::ast::predule::{
    BinaryOperator, BinaryOperatorExpression, SQLExpression, UnaryOperator, UnaryOperatorExpression,
};
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
            Token::Operator(operator) => {
                if operator.is_unary_operator() {
                    let expression = self.parse_expression()?;
                    let operator: UnaryOperator = operator.try_into()?;
                    return Ok(UnaryOperatorExpression {
                        operand: expression,
                        operator,
                    }
                    .into());
                } else {
                    return Err(ParsingError::boxed(format!(
                        "unexpected operator: {:?}",
                        operator
                    )));
                }
            }
            Token::Integer(integer) => {
                let lhs = SQLExpression::Integer(integer);

                if self.next_token_is_binary_operator() {
                    let expression = self.parse_binary_expression(lhs)?;
                    return Ok(expression);
                } else {
                    return Ok(lhs);
                }
            }
            Token::Float(float) => {
                let lhs = SQLExpression::Float(float);

                if self.next_token_is_binary_operator() {
                    let expression = self.parse_binary_expression(lhs)?;
                    return Ok(expression);
                } else {
                    return Ok(lhs);
                }
            }
            Token::Identifier(identifier) => {
                self.unget_next_token(Token::Identifier(identifier));
                let select_column = self.parse_select_column()?;

                let lhs = SQLExpression::SelectColumn(select_column);

                if self.next_token_is_binary_operator() {
                    let expression = self.parse_binary_expression(lhs)?;
                    return Ok(expression);
                } else {
                    return Ok(lhs);
                }
            }
            Token::String(string) => {
                let lhs = SQLExpression::String(string);

                if self.next_token_is_binary_operator() {
                    let expression = self.parse_binary_expression(lhs)?;
                    return Ok(expression);
                } else {
                    return Ok(lhs);
                }
            }
            Token::Boolean(boolean) => {
                let lhs = SQLExpression::Boolean(boolean);

                if self.next_token_is_binary_operator() {
                    let expression = self.parse_binary_expression(lhs)?;
                    return Ok(expression);
                } else {
                    return Ok(lhs);
                }
            }
            Token::LeftParentheses => {
                self.unget_next_token(current_token);
                let expression = self.parse_parentheses_expression()?;

                return Ok(expression);
            }
            Token::RightParentheses => {
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
     * ????????? ??????
    parenexpr ::= '(' expression ')'
    */
    pub(crate) fn parse_parentheses_expression(&mut self) -> Result<SQLExpression, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // ( ??????
        let current_token = self.get_next_token();

        if current_token != Token::LeftParentheses {
            return Err(ParsingError::boxed(format!(
                "expected left parentheses. but your input is {:?}",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // ????????? ??????
        let expression = self.parse_expression()?;

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // ) ??????
        let current_token = self.get_next_token();

        if current_token != Token::RightParentheses {
            return Err(ParsingError::boxed(format!(
                "expected right parentheses. but your input is {:?}",
                current_token
            )));
        }

        Ok(expression)
    }

    /**
     * 2??? ????????? ??????
     */
    pub(crate) fn parse_binary_expression(
        &mut self,
        lhs: SQLExpression,
    ) -> Result<SQLExpression, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // ????????? ??????
        let current_token = self.get_next_token();

        match current_token {
            Token::Operator(operator) => {
                if operator.is_binary_operator() {
                    let rhs = self.parse_expression()?;
                    let operator: BinaryOperator = operator.try_into()?;

                    let current_precedence = operator.get_precedence();

                    if let SQLExpression::Binary(rhs_binary) = rhs.clone() {
                        let next_precedence = rhs_binary.operator.get_precedence();

                        if next_precedence > current_precedence {
                            return Ok(BinaryOperatorExpression { lhs, rhs, operator }.into());
                        } else {
                            let new_lhs = BinaryOperatorExpression {
                                lhs,
                                rhs: rhs_binary.lhs,
                                operator,
                            };
                            return Ok(BinaryOperatorExpression {
                                lhs: new_lhs.into(),
                                rhs: rhs_binary.rhs,
                                operator: rhs_binary.operator,
                            }
                            .into());
                        }
                    } else {
                        return Ok(BinaryOperatorExpression { lhs, rhs, operator }.into());
                    }
                } else {
                    return Err(ParsingError::boxed(format!(
                        "binary operator expected, but your input is {:?}",
                        operator
                    )));
                }
            }
            Token::And | Token::Or => {
                return Err(ParsingError::boxed(format!("not implement")));
            }
            _ => {
                return Err(ParsingError::boxed(format!(
                    "operator is expected, but your input is {:?}",
                    current_token
                )));
            }
        }
    }
}
