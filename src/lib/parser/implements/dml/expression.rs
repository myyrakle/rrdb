use std::convert::TryInto;
use std::error::Error;

use crate::lib::ast::predule::{
    BinaryOperator, BinaryOperatorExpression, CallExpression, FunctionName, ParenthesesExpression,
    SQLExpression, UnaryOperator, UnaryOperatorExpression,
};
use crate::lib::errors::predule::ParsingError;
use crate::lib::lexer::predule::Token;
use crate::lib::parser::predule::Parser;
use crate::lib::types::SelectColumn;

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
                    let lhs: SQLExpression = UnaryOperatorExpression {
                        operand: expression,
                        operator,
                    }
                    .into();

                    if self.next_token_is_binary_operator() {
                        let expression = self.parse_binary_expression(lhs)?;
                        return Ok(expression);
                    } else {
                        return Ok(lhs);
                    }
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

                let lhs = SQLExpression::SelectColumn(select_column.clone());

                if self.next_token_is_binary_operator() {
                    let expression = self.parse_binary_expression(lhs)?;
                    return Ok(expression);
                } else if self.next_token_is_left_parentheses() {
                    let SelectColumn {
                        table_name,
                        column_name,
                    } = select_column;

                    let expression =
                        self.parse_function_call_expression(table_name, column_name)?;
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
            Token::Null => {
                let lhs = SQLExpression::Null;

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
     * 소괄호 파싱
    parenexpr ::= '(' expression ')'
    */
    pub(crate) fn parse_parentheses_expression(&mut self) -> Result<SQLExpression, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // ( 삼킴
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

        // 표현식 파싱
        let expression = self.parse_expression()?;

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // ) 삼킴
        let current_token = self.get_next_token();

        if current_token != Token::RightParentheses {
            return Err(ParsingError::boxed(format!(
                "expected right parentheses. but your input is {:?}",
                current_token
            )));
        }

        let expression = ParenthesesExpression { expression };

        Ok(expression.into())
    }

    /**
     * 2항 연산식 파싱
     */
    pub(crate) fn parse_binary_expression(
        &mut self,
        lhs: SQLExpression,
    ) -> Result<SQLExpression, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // 연산자 획득
        let current_token = self.get_next_token();
        let operator: Result<BinaryOperator, _> = current_token.try_into();

        match operator {
            Ok(operator) => {
                let rhs = self.parse_expression()?;

                let current_precedence = operator.get_precedence();

                let mut rhs_has_parentheses = false;

                // 소괄호가 있다면 벗기고 플래그값 설정
                let rhs = if let SQLExpression::Parentheses(paren) = rhs {
                    rhs_has_parentheses = true;
                    paren.expression
                } else {
                    rhs
                };

                if let SQLExpression::Binary(rhs_binary) = rhs.clone() {
                    let next_precedence = rhs_binary.operator.get_precedence();

                    // 오른쪽 연산자의 우선순위가 더 크거나, 소괄호가 있을 경우 오른쪽을 먼저 묶어서 바인딩
                    if next_precedence > current_precedence || rhs_has_parentheses {
                        return Ok(BinaryOperatorExpression { lhs, rhs, operator }.into());
                    }
                    // 아니라면 왼쪽으로 묶어서 바인딩
                    else {
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
            }
            Err(error) => {
                return Err(error);
            }
        }
    }

    /**
     * 함수호출 파싱
     */
    pub(crate) fn parse_function_call_expression(
        &mut self,
        database_name: Option<String>,
        function_name: String,
    ) -> Result<SQLExpression, Box<dyn Error>> {
        let mut call_expression = CallExpression {
            function_name: FunctionName {
                database_name,
                function_name,
            },
            arguments: vec![],
        };

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // ( 삼킴
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

        // 닫는 괄호가 나올때까지 인자 파싱
        loop {
            if self.next_token_is_right_parentheses() {
                break;
            }

            // 표현식 파싱
            let expression = self.parse_expression()?;

            call_expression.arguments.push(expression);

            // 쉼표 삼키기.
            if self.next_token_is_comma() {
                self.get_next_token();
            }
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // ) 삼킴
        let current_token = self.get_next_token();

        if current_token != Token::RightParentheses {
            return Err(ParsingError::boxed(format!(
                "expected right parentheses. but your input is {:?}",
                current_token
            )));
        }

        Ok(call_expression.into())
    }
}
