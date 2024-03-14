use std::convert::{TryFrom, TryInto};
use std::error::Error;

use crate::ast::predule::{
    BetweenExpression, BinaryOperator, BinaryOperatorExpression, BuiltInFunction, CallExpression,
    ListExpression, NotBetweenExpression, ParenthesesExpression, SQLExpression, SelectColumn,
    UnaryOperator, UnaryOperatorExpression, UserDefinedFunction,
};
use crate::errors::predule::ParsingError;
use crate::lexer::predule::Token;
use crate::parser::predule::Parser;
use crate::parser::predule::ParserContext;

impl Parser {
    pub(crate) fn parse_expression(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLExpression, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0201 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Operator(operator) => {
                if operator.is_unary_operator() {
                    let operator: UnaryOperator = operator.try_into()?;
                    let expression = self.parse_unary_expression(operator, context)?;

                    Ok(expression)
                } else {
                    Err(ParsingError::boxed(format!(
                        "E0212 unexpected operator: {:?}",
                        operator
                    )))
                }
            }
            Token::Not => {
                let operator = UnaryOperator::Not;

                let expression = self.parse_unary_expression(operator, context)?;

                Ok(expression)
            }
            Token::Integer(integer) => {
                let lhs = SQLExpression::Integer(integer);

                if self.next_token_is_binary_operator(context.clone()) {
                    let expression = self.parse_binary_expression(lhs, context)?;
                    Ok(expression)
                } else if self.next_token_is_between() {
                    let expression = self.parse_between_expression(lhs, context)?;
                    Ok(expression)
                } else {
                    Ok(lhs)
                }
            }
            Token::Float(float) => {
                let lhs = SQLExpression::Float(float);

                if self.next_token_is_binary_operator(context.clone()) {
                    let expression = self.parse_binary_expression(lhs, context)?;
                    Ok(expression)
                } else if self.next_token_is_between() {
                    let expression = self.parse_between_expression(lhs, context)?;
                    Ok(expression)
                } else {
                    Ok(lhs)
                }
            }
            Token::String(string) => {
                let lhs = SQLExpression::String(string);

                if self.next_token_is_binary_operator(context.clone()) {
                    let expression = self.parse_binary_expression(lhs, context)?;
                    Ok(expression)
                } else if self.next_token_is_between() {
                    let expression = self.parse_between_expression(lhs, context)?;
                    Ok(expression)
                } else {
                    Ok(lhs)
                }
            }
            Token::Boolean(boolean) => {
                let lhs = SQLExpression::Boolean(boolean);

                if self.next_token_is_binary_operator(context.clone()) {
                    let expression = self.parse_binary_expression(lhs, context)?;
                    Ok(expression)
                } else if self.next_token_is_between() {
                    let expression = self.parse_between_expression(lhs, context)?;
                    Ok(expression)
                } else {
                    Ok(lhs)
                }
            }
            Token::Null => {
                let lhs = SQLExpression::Null;

                if self.next_token_is_binary_operator(context.clone()) {
                    let expression = self.parse_binary_expression(lhs, context)?;
                    Ok(expression)
                } else if self.next_token_is_between() {
                    let expression = self.parse_between_expression(lhs, context)?;
                    Ok(expression)
                } else {
                    Ok(lhs)
                }
            }
            Token::LeftParentheses => {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E0214 need more tokens"));
                }

                let second_token = self.get_next_token();

                match second_token {
                    Token::Select => {
                        self.unget_next_token(second_token);
                        self.unget_next_token(current_token);
                        let lhs = self.parse_subquery(context.clone())?.into();

                        if self.next_token_is_binary_operator(context.clone()) {
                            let expression = self.parse_binary_expression(lhs, context)?;
                            Ok(expression)
                        } else if self.next_token_is_between() {
                            let expression = self.parse_between_expression(lhs, context)?;
                            Ok(expression)
                        } else {
                            Ok(lhs)
                        }
                    }
                    _ => {
                        self.unget_next_token(second_token);
                        self.unget_next_token(current_token);
                        let lhs = self.parse_parentheses_expression(context.clone())?;

                        if self.next_token_is_binary_operator(context.clone()) {
                            let expression = self.parse_binary_expression(lhs, context)?;
                            Ok(expression)
                        } else if self.next_token_is_between() {
                            let expression = self.parse_between_expression(lhs, context)?;
                            Ok(expression)
                        } else {
                            Ok(lhs)
                        }
                    }
                }
            }
            Token::RightParentheses => Err(ParsingError::boxed(format!(
                "E0213 unexpected token: {:?}",
                current_token
            ))),
            Token::Identifier(identifier) => {
                self.unget_next_token(Token::Identifier(identifier));
                let select_column = self.parse_select_column()?;

                let lhs = SQLExpression::SelectColumn(select_column.clone());

                if self.next_token_is_binary_operator(context.clone()) {
                    let expression = self.parse_binary_expression(lhs, context)?;
                    Ok(expression)
                } else if self.next_token_is_between() {
                    let expression = self.parse_between_expression(lhs, context)?;
                    Ok(expression)
                } else if self.next_token_is_left_parentheses() {
                    let SelectColumn {
                        table_name,
                        column_name,
                    } = select_column;

                    let lhs = self.parse_function_call_expression(
                        table_name,
                        column_name,
                        context.clone(),
                    )?;

                    if self.next_token_is_binary_operator(context.clone()) {
                        let expression = self.parse_binary_expression(lhs, context)?;
                        Ok(expression)
                    } else if self.next_token_is_between() {
                        let expression = self.parse_between_expression(lhs, context)?;
                        Ok(expression)
                    } else {
                        Ok(lhs)
                    }
                } else {
                    Ok(lhs)
                }
            }
            _ => Err(ParsingError::boxed(format!(
                "E0202 unexpected token: {:?}",
                current_token
            ))),
        }
    }

    pub(crate) fn parse_unary_expression(
        &mut self,
        operator: UnaryOperator,
        context: ParserContext,
    ) -> Result<SQLExpression, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0201 need more tokens"));
        }

        let expression = self.parse_expression(context)?;

        // expression이 2항 표현식일 경우 단항 표현식이 최우선으로 처리되게 구성
        match expression {
            SQLExpression::Binary(mut binary) => {
                binary.lhs = UnaryOperatorExpression {
                    operand: binary.lhs,
                    operator,
                }
                .into();

                Ok(binary.into())
            }
            SQLExpression::Between(mut between) => {
                between.a = UnaryOperatorExpression {
                    operand: between.a,
                    operator,
                }
                .into();

                Ok(between.into())
            }
            _ => Ok(UnaryOperatorExpression {
                operand: expression,
                operator,
            }
            .into()),
        }
    }

    /**
     * 소괄호연산자, 혹은 리스트 파싱
    parenexpr ::= '(' expression ')'
    parenexpr ::= '(' 1, 2, 3 ')'
    */
    pub(crate) fn parse_parentheses_expression(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLExpression, Box<dyn Error + Send>> {
        let context = context.set_in_parentheses(true);

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0203 need more tokens"));
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
            return Err(ParsingError::boxed("E0204 need more tokens"));
        }

        // 표현식 파싱
        let expression = self.parse_expression(context.clone())?;

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0205 need more tokens"));
        }

        // ) 삼킴
        let current_token = self.get_next_token();

        match current_token {
            // 우선순위 연산자
            Token::RightParentheses => {
                let expression = ParenthesesExpression { expression };

                Ok(expression.into())
            }
            // 리스트 표현식
            Token::Comma => {
                let mut list = ListExpression {
                    value: vec![expression],
                };

                loop {
                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("E0215 need more tokens"));
                    }

                    let current_token = self.get_next_token();

                    match current_token {
                        Token::RightParentheses => break,
                        Token::Comma => continue,
                        _ => {
                            self.unget_next_token(current_token);
                            let expression = self.parse_expression(context.clone())?;
                            list.value.push(expression);
                            continue;
                        }
                    }
                }

                Ok(list.into())
            }
            _ => Err(ParsingError::boxed(format!(
                "expected right parentheses. but your input is {:?}",
                current_token
            ))),
        }
    }

    /**
     * 2항 연산식 파싱
     */
    pub(crate) fn parse_binary_expression(
        &mut self,
        lhs: SQLExpression,
        context: ParserContext,
    ) -> Result<SQLExpression, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0206 need more tokens"));
        }

        // 연산자 획득
        let current_token = self.get_next_token();

        let operator: Result<BinaryOperator, _> =
            if current_token.can_be_multi_token_operator() && self.has_next_token() {
                let second_token = self.get_next_token();
                current_token.try_into_multi_token_operator(second_token)
            } else {
                current_token.try_into()
            };

        match operator {
            Ok(operator) => {
                let rhs = self.parse_expression(context)?;

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

                    // 단항연산식일 경우
                    if lhs.is_unary() {
                        let new_lhs = BinaryOperatorExpression {
                            lhs,
                            rhs: rhs_binary.lhs,
                            operator,
                        };
                        Ok(BinaryOperatorExpression {
                            lhs: new_lhs.into(),
                            rhs: rhs_binary.rhs,
                            operator: rhs_binary.operator,
                        }
                        .into())
                    }
                    // 2항연산식일 경우
                    else {
                        // 오른쪽 연산자의 우선순위가 더 크거나, 소괄호가 있을 경우 오른쪽을 먼저 묶어서 바인딩
                        if next_precedence > current_precedence || rhs_has_parentheses {
                            Ok(BinaryOperatorExpression { lhs, rhs, operator }.into())
                        }
                        // 아니라면 왼쪽으로 묶어서 바인딩
                        else {
                            let new_lhs = BinaryOperatorExpression {
                                lhs,
                                rhs: rhs_binary.lhs,
                                operator,
                            };
                            Ok(BinaryOperatorExpression {
                                lhs: new_lhs.into(),
                                rhs: rhs_binary.rhs,
                                operator: rhs_binary.operator,
                            }
                            .into())
                        }
                    }
                } else {
                    Ok(BinaryOperatorExpression { lhs, rhs, operator }.into())
                }
            }
            Err(error) => Err(error),
        }
    }

    /**
     * 함수호출 파싱
     */
    pub(crate) fn parse_function_call_expression(
        &mut self,
        database_name: Option<String>,
        function_name: String,
        context: ParserContext,
    ) -> Result<SQLExpression, Box<dyn Error + Send>> {
        let function = if database_name.is_some() {
            UserDefinedFunction {
                database_name,
                function_name,
            }
            .into()
        } else {
            match BuiltInFunction::try_from(function_name.clone()) {
                Ok(builtin) => builtin.into(),
                Err(_) => UserDefinedFunction {
                    database_name,
                    function_name,
                }
                .into(),
            }
        };

        let mut call_expression = CallExpression {
            function,
            arguments: vec![],
        };

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0207 need more tokens"));
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
            return Err(ParsingError::boxed("E0208 need more tokens"));
        }

        // 닫는 괄호가 나올때까지 인자 파싱
        loop {
            if self.next_token_is_right_parentheses() {
                break;
            }

            // 표현식 파싱
            let expression = self.parse_expression(context.clone())?;

            call_expression.arguments.push(expression);

            // 쉼표 삼키기.
            if self.next_token_is_comma() {
                self.get_next_token();
            }
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0209 need more tokens"));
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

    /**
     * between 및 not between 절 파싱
     */
    pub(crate) fn parse_between_expression(
        &mut self,
        a: SQLExpression,
        context: ParserContext,
    ) -> Result<SQLExpression, Box<dyn Error + Send>> {
        let context = context.set_in_between_clause(true);

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0210 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Between => {
                let x = self.parse_expression(context.clone())?;

                // AND 삼킴
                self.get_next_token();

                let y = self.parse_expression(context)?;

                let expression = BetweenExpression { a, x, y };

                Ok(expression.into())
            }
            Token::Not => {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E0211 need more tokens"));
                }

                let current_token = self.get_next_token();

                match current_token {
                    Token::Between => {
                        let x = self.parse_expression(context.clone())?;

                        // AND 삼킴
                        self.get_next_token();

                        let y = self.parse_expression(context)?;

                        let expression = NotBetweenExpression { a, x, y };

                        Ok(expression.into())
                    }
                    _ => Err(ParsingError::boxed(format!(
                        "expected between. but your input is {:?}",
                        current_token
                    ))),
                }
            }
            _ => Err(ParsingError::boxed(format!(
                "expected between. but your input is {:?}",
                current_token
            ))),
        }
    }
}
