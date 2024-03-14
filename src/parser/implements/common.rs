use std::error::Error;

use crate::ast::predule::{
    Column, DataType, JoinType, SelectColumn, SubqueryExpression, TableName,
};
use crate::errors::predule::ParsingError;
use crate::lexer::predule::{OperatorToken, Token};
use crate::parser::predule::{Parser, ParserContext};

impl Parser {
    // 테이블 컬럼 정의 분석
    pub(crate) fn parse_table_column(&mut self) -> Result<Column, Box<dyn Error + Send>> {
        let mut builder = Column::builder();

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0001 need more tokens"));
        }

        let current_token = self.get_next_token();

        if let Token::Identifier(name) = current_token {
            builder = builder.set_name(name);
        } else {
            return Err(ParsingError::boxed(format!(
                "E0028 expected identifier. but your input word is '{:?}'",
                current_token
            )));
        }

        let data_type = self.parse_data_type()?;
        builder = builder.set_data_type(data_type);

        loop {
            if !self.has_next_token() {
                break;
            }

            let current_token = self.get_next_token();

            match current_token {
                Token::Comma => {
                    // , 만나면 종료
                    break;
                }
                Token::RightParentheses => {
                    // ) 만나면 종료
                    self.unget_next_token(current_token);
                    break;
                }
                Token::Primary => {
                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("E0003 need more tokens"));
                    }

                    let current_token = self.get_next_token();

                    match current_token {
                        Token::Key => {
                            builder = builder.set_primary_key(true).set_not_null(true);
                        }
                        _ => {
                            return Err(ParsingError::boxed(format!(
                                "expected 'PRIMARY KEY'. but your input word is '{:?}'",
                                current_token
                            )));
                        }
                    }
                }
                Token::Not => {
                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("E0004 need more tokens"));
                    }

                    let current_token = self.get_next_token();

                    match current_token {
                        Token::Null => {
                            builder = builder.set_not_null(true);
                        }
                        _ => {
                            return Err(ParsingError::boxed(format!(
                                "expected 'NOT NULL'. but your input word is '{:?}'",
                                current_token
                            )));
                        }
                    }
                }
                Token::Null => {
                    builder = builder.set_not_null(false);
                }
                Token::Comment => {
                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("E0005 need more tokens"));
                    }

                    let current_token = self.get_next_token();

                    if let Token::String(comment) = current_token {
                        builder = builder.set_comment(comment);
                    } else {
                        return Err(ParsingError::boxed(format!(
                            "expected comment string. but your input word is '{:?}'",
                            current_token
                        )));
                    }
                }
                Token::Default => {
                    return Err(ParsingError::boxed("not supported yet"));
                }
                _ => {}
            }
        }

        Ok(builder.build())
    }

    // 데이터 타입 분석
    pub(crate) fn parse_data_type(&mut self) -> Result<DataType, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0006 need more tokens"));
        }

        let current_token = self.get_next_token();

        if let Token::Identifier(type_name) = current_token {
            match type_name.to_uppercase().as_str() {
                "INTEGER" | "INT" => Ok(DataType::Int),
                "FLOAT" => Ok(DataType::Float),
                "BOOLEAN" | "BOOL" => Ok(DataType::Boolean),
                "VARCHAR" => {
                    // 여는 괄호 체크
                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("E0007 need more tokens"));
                    }

                    let current_token = self.get_next_token();

                    if Token::LeftParentheses != current_token {
                        return Err(ParsingError::boxed(format!(
                            "expected '('. but your input word is '{:?}'",
                            current_token
                        )));
                    }

                    // 문자열 길이 체크
                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("E0008 need more tokens"));
                    }

                    let current_token = self.get_next_token();

                    if let Token::Integer(integer) = current_token {
                        // 닫는 괄호 체크
                        if !self.has_next_token() {
                            return Err(ParsingError::boxed("E0009 need more tokens"));
                        }

                        let current_token = self.get_next_token();

                        if Token::RightParentheses != current_token {
                            return Err(ParsingError::boxed(format!(
                                "expected ')'. but your input word is '{:?}'",
                                current_token
                            )));
                        }

                        Ok(DataType::Varchar(integer))
                    } else {
                        Err(ParsingError::boxed(format!(
                            "expected integer number. but your input word is '{:?}'",
                            current_token
                        )))
                    }
                }
                _ => Err(ParsingError::boxed(format!(
                    "unknown data type '{}'",
                    type_name
                ))),
            }
        } else {
            Err(ParsingError::boxed(format!(
                "E0029 expected identifier. but your input word is '{:?}'",
                current_token
            )))
        }
    }

    // 테이블명 분석
    pub(crate) fn parse_table_name(
        &mut self,
        context: ParserContext,
    ) -> Result<TableName, Box<dyn Error + Send>> {
        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0010 need more tokens"));
        }

        // 첫번째로 오는 이름은 테이블명으로 추정
        let current_token = self.get_next_token();
        let mut database_name = None;

        let mut table_name = if let Token::Identifier(name) = current_token {
            name
        } else {
            return Err(ParsingError::boxed(format!(
                "E0030 expected identifier. but your input word is '{:?}'",
                current_token
            )));
        };

        if !self.has_next_token() {
            return Ok(TableName::new(
                database_name.or(context.default_database),
                table_name,
            ));
        }

        let current_token = self.get_next_token();

        // .가 있을 경우 "데이터베이스명"."테이블명"의 형태로 추정
        if current_token == Token::Period {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("E0012 need more tokens"));
            }

            let current_token = self.get_next_token();

            if let Token::Identifier(name) = current_token {
                database_name = Some(table_name);
                table_name = name;
            } else {
                return Err(ParsingError::boxed(format!(
                    "E0031 expected identifier. but your input word is '{:?}'",
                    current_token
                )));
            }
        } else {
            self.unget_next_token(current_token);
        }

        Ok(TableName::new(
            database_name.or(context.default_database),
            table_name,
        ))
    }

    // IF NOT EXISTS 체크 로직
    pub(crate) fn has_if_not_exists(&mut self) -> Result<bool, Box<dyn Error + Send>> {
        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0013 need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::If == current_token {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("E0014 need more tokens"));
            }

            let current_token = self.get_next_token();

            if Token::Not == current_token {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E0015 need more tokens"));
                }

                let current_token = self.get_next_token();

                if Token::Exists == current_token {
                    Ok(true)
                } else {
                    Err(ParsingError::boxed(format!(
                        "expected keyword is 'exists'. but your input word is '{:?}'",
                        current_token
                    )))
                }
            } else {
                Err(ParsingError::boxed(format!(
                    "expected keyword is 'not'. but your input word is '{:?}'",
                    current_token
                )))
            }
        } else {
            self.unget_next_token(current_token);
            Ok(false)
        }
    }

    // IF EXISTS 체크 로직
    pub(crate) fn has_if_exists(&mut self) -> Result<bool, Box<dyn Error + Send>> {
        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0016 need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::If == current_token {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("E0017 need more tokens"));
            }

            let current_token = self.get_next_token();

            if Token::Exists == current_token {
                Ok(true)
            } else {
                Err(ParsingError::boxed(format!(
                    "expected keyword is 'exists'. but your input word is '{:?}'",
                    current_token
                )))
            }
        } else {
            self.unget_next_token(current_token);
            Ok(false)
        }
    }

    // SELECT 컬럼 정의 분석
    pub(crate) fn parse_select_column(&mut self) -> Result<SelectColumn, Box<dyn Error + Send>> {
        let mut select_column = SelectColumn::new(None, "".to_string());

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0018 need more tokens"));
        }

        let current_token = self.get_next_token();

        if let Token::Identifier(name) = current_token {
            select_column.column_name = name;
        } else {
            return Err(ParsingError::boxed(format!(
                "E0032 expected identifier. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            Ok(select_column)
        } else {
            let current_token = self.get_next_token();

            if current_token == Token::Period {
                let current_token = self.get_next_token();

                if let Token::Identifier(name) = current_token {
                    select_column.table_name = Some(select_column.column_name);
                    select_column.column_name = name;
                    Ok(select_column)
                } else {
                    Err(ParsingError::boxed(format!(
                        "E0033 expected identifier. but your input word is '{:?}'",
                        current_token
                    )))
                }
            } else {
                self.unget_next_token(current_token);
                Ok(select_column)
            }
        }
    }

    // 다음 토큰이 2항 연산자/키워드인지
    pub(crate) fn next_token_is_binary_operator(&mut self, context: ParserContext) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            // 2항 키워드, 연산자일 경우에만 true 반환
            match current_token {
                Token::And => {
                    self.unget_next_token(current_token);

                    // BETWEEN 파싱중이면서 괄호가 없는 상태라면 연산자가 아닌 것으로 간주.
                    #[allow(clippy::needless_bool)]
                    if context.in_between_clause && !context.in_parentheses {
                        false
                    } else {
                        true
                    }
                }
                Token::Or | Token::Like | Token::In => {
                    self.unget_next_token(current_token);
                    true
                }
                Token::Operator(ref operator) => {
                    let result = [
                        OperatorToken::Plus,
                        OperatorToken::Minus,
                        OperatorToken::Asterisk,
                        OperatorToken::Slash,
                        OperatorToken::Lt,
                        OperatorToken::Lte,
                        OperatorToken::Gt,
                        OperatorToken::Gte,
                        OperatorToken::Eq,
                        OperatorToken::Neq,
                    ]
                    .contains(operator);

                    self.unget_next_token(current_token);

                    result
                }
                Token::Is => {
                    self.unget_next_token(current_token);
                    true
                }
                Token::Not => {
                    if !self.has_next_token() {
                        self.unget_next_token(current_token);
                        false
                    } else {
                        let second_token = self.get_next_token();

                        match second_token {
                            Token::In | Token::Like => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(current_token);
                                true
                            }
                            _ => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(current_token);
                                false
                            }
                        }
                    }
                }
                _ => {
                    self.unget_next_token(current_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 여는 괄호인지
    pub(crate) fn next_token_is_left_parentheses(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            self.unget_next_token(current_token.clone());

            current_token == Token::LeftParentheses
        }
    }

    // 다음 토큰이 여는 괄호인지
    pub(crate) fn _next_token_is_subquery(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            if current_token == Token::LeftParentheses {
                if !self.has_next_token() {
                    self.unget_next_token(current_token);
                    false
                } else {
                    let second_token = self.get_next_token();

                    if second_token == Token::Select {
                        self.unget_next_token(second_token);
                        self.unget_next_token(current_token);
                        true
                    } else {
                        self.unget_next_token(second_token);
                        self.unget_next_token(current_token);
                        false
                    }
                }
            } else {
                self.unget_next_token(current_token);
                false
            }
        }
    }

    // 다음 토큰이 닫는 괄호인지
    pub(crate) fn next_token_is_right_parentheses(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            self.unget_next_token(current_token.clone());

            current_token == Token::RightParentheses
        }
    }

    // 다음 토큰이 쉼표인지
    pub(crate) fn next_token_is_comma(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            self.unget_next_token(current_token.clone());

            current_token == Token::Comma
        }
    }

    // 다음 토큰이 여는 괄호인지
    pub(crate) fn next_token_is_between(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            match current_token {
                Token::Between => {
                    self.unget_next_token(current_token);
                    true
                }
                Token::Not => {
                    if !self.has_next_token() {
                        self.unget_next_token(current_token);
                        false
                    } else {
                        let second_token = self.get_next_token();
                        match second_token.clone() {
                            Token::Between => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(current_token);
                                true
                            }
                            _ => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(current_token);
                                false
                            }
                        }
                    }
                }
                _ => {
                    self.unget_next_token(current_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 AS인지
    pub(crate) fn next_token_is_table_alias(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            match current_token.clone() {
                Token::As => {
                    self.unget_next_token(current_token);
                    true
                }
                Token::Identifier(_) => {
                    self.unget_next_token(current_token);
                    true
                }
                _ => {
                    self.unget_next_token(current_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 AS인지
    pub(crate) fn next_token_is_where(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            match current_token.clone() {
                Token::Where => {
                    self.unget_next_token(current_token);
                    true
                }
                _ => {
                    self.unget_next_token(current_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 ORDER BY인지
    pub(crate) fn next_token_is_order_by(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            match current_token {
                Token::Order => {
                    if !self.has_next_token() {
                        self.unget_next_token(current_token);
                        return false;
                    }

                    let second_token = self.get_next_token();

                    match second_token {
                        Token::By => {
                            self.unget_next_token(second_token);
                            self.unget_next_token(current_token);
                            true
                        }
                        _ => {
                            self.unget_next_token(second_token);
                            self.unget_next_token(current_token);
                            false
                        }
                    }
                }
                _ => {
                    self.unget_next_token(current_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 GROUP BY인지
    pub(crate) fn next_token_is_group_by(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            match current_token {
                Token::Group => {
                    if !self.has_next_token() {
                        self.unget_next_token(current_token);
                        return false;
                    }

                    let second_token = self.get_next_token();

                    match second_token {
                        Token::By => {
                            self.unget_next_token(second_token);
                            self.unget_next_token(current_token);
                            true
                        }
                        _ => {
                            self.unget_next_token(second_token);
                            self.unget_next_token(current_token);
                            false
                        }
                    }
                }
                _ => {
                    self.unget_next_token(current_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 HAVING인지
    pub(crate) fn next_token_is_having(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            match current_token {
                Token::Having => {
                    self.unget_next_token(current_token);
                    true
                }
                _ => {
                    self.unget_next_token(current_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 OFFSET인지
    pub(crate) fn next_token_is_offset(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            match current_token {
                Token::Offset => {
                    self.unget_next_token(current_token);
                    true
                }
                _ => {
                    self.unget_next_token(current_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 OFFSET인지
    pub(crate) fn next_token_is_limit(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            match current_token {
                Token::Limit => {
                    self.unget_next_token(current_token);
                    true
                }
                _ => {
                    self.unget_next_token(current_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 COLUMN인지
    pub(crate) fn next_token_is_column(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let current_token = self.get_next_token();

            match current_token {
                Token::Column => {
                    self.unget_next_token(current_token);
                    true
                }
                _ => {
                    self.unget_next_token(current_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 COLUMN인지
    pub(crate) fn next_token_is_not_null(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let first_token = self.get_next_token();

            match first_token {
                Token::Not => {
                    if !self.has_next_token() {
                        self.unget_next_token(first_token);
                        false
                    } else {
                        let second_token = self.get_next_token();

                        match second_token {
                            Token::Null => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(first_token);
                                true
                            }
                            _ => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(first_token);
                                false
                            }
                        }
                    }
                }
                _ => {
                    self.unget_next_token(first_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 COLUMN인지
    pub(crate) fn next_token_is_data_type(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let first_token = self.get_next_token();

            match first_token {
                Token::Data => {
                    if !self.has_next_token() {
                        self.unget_next_token(first_token);
                        false
                    } else {
                        let second_token = self.get_next_token();

                        match second_token {
                            Token::Type => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(first_token);
                                true
                            }
                            _ => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(first_token);
                                false
                            }
                        }
                    }
                }
                _ => {
                    self.unget_next_token(first_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 COLUMN인지
    pub(crate) fn next_token_is_default(&mut self) -> bool {
        if !self.has_next_token() {
            false
        } else {
            let first_token = self.get_next_token();

            match first_token {
                Token::Default => {
                    self.unget_next_token(first_token);
                    true
                }
                _ => {
                    self.unget_next_token(first_token);
                    false
                }
            }
        }
    }

    // 다음 토큰이 JOIN 토큰이라면 JOIN 타입을 추출해서 반환
    pub(crate) fn get_next_join_type(&mut self) -> Option<JoinType> {
        if !self.has_next_token() {
            None
        } else {
            let current_token = self.get_next_token();

            match current_token {
                Token::Inner => {
                    if !self.has_next_token() {
                        self.unget_next_token(current_token);
                        None
                    } else {
                        let second_token = self.get_next_token();

                        match second_token {
                            Token::Join => Some(JoinType::InnerJoin),
                            _ => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(current_token);
                                None
                            }
                        }
                    }
                }
                Token::Left | Token::Right => {
                    if !self.has_next_token() {
                        self.unget_next_token(current_token);
                        None
                    } else {
                        let second_token = self.get_next_token();

                        match second_token {
                            Token::Join => match current_token {
                                Token::Left => Some(JoinType::LeftOuterJoin),
                                Token::Right => Some(JoinType::RightOuterJoin),
                                _ => unreachable!(),
                            },
                            Token::Outer => {
                                if !self.has_next_token() {
                                    self.unget_next_token(second_token);
                                    self.unget_next_token(current_token);
                                    None
                                } else {
                                    let third_token = self.get_next_token();

                                    match third_token {
                                        Token::Join => match current_token {
                                            Token::Left => Some(JoinType::LeftOuterJoin),
                                            Token::Right => Some(JoinType::RightOuterJoin),
                                            _ => unreachable!(),
                                        },
                                        _ => {
                                            self.unget_next_token(third_token);
                                            self.unget_next_token(second_token);
                                            self.unget_next_token(current_token);
                                            None
                                        }
                                    }
                                }
                            }
                            _ => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(current_token);
                                None
                            }
                        }
                    }
                }
                Token::Full => {
                    if !self.has_next_token() {
                        self.unget_next_token(current_token);
                        None
                    } else {
                        let second_token = self.get_next_token();

                        match second_token {
                            Token::Join => Some(JoinType::FullOuterJoin),
                            Token::Outer => {
                                if !self.has_next_token() {
                                    self.unget_next_token(current_token);
                                    None
                                } else {
                                    let third_token = self.get_next_token();

                                    match third_token {
                                        Token::Join => Some(JoinType::FullOuterJoin),
                                        _ => {
                                            self.unget_next_token(third_token);
                                            self.unget_next_token(second_token);
                                            self.unget_next_token(current_token);
                                            None
                                        }
                                    }
                                }
                            }
                            _ => {
                                self.unget_next_token(second_token);
                                self.unget_next_token(current_token);
                                None
                            }
                        }
                    }
                }
                Token::Join => Some(JoinType::InnerJoin),
                _ => {
                    self.unget_next_token(current_token);
                    None
                }
            }
        }
    }

    // Table Alias 획득
    pub(crate) fn parse_table_alias(&mut self) -> Result<String, Box<dyn Error + Send>> {
        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0024 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::As => {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E0026 need more tokens"));
                }

                let current_token = self.get_next_token();

                match current_token {
                    Token::Identifier(id) => Ok(id),
                    _ => Err(ParsingError::boxed(format!(
                        "E0027 expected identifier. but your input is {:?}",
                        current_token
                    ))),
                }
            }
            Token::Identifier(id) => Ok(id),
            _ => Err(ParsingError::boxed(format!(
                "E0025 expected AS. but your input is {:?}",
                current_token
            ))),
        }
    }

    // 서브쿼리 분석
    pub(crate) fn parse_subquery(
        &mut self,
        context: ParserContext,
    ) -> Result<SubqueryExpression, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0019 need more tokens"));
        }

        // ( 삼킴
        let current_token = self.get_next_token();

        if current_token != Token::LeftParentheses {
            return Err(ParsingError::boxed(format!(
                "E0020 expected left parentheses. but your input is {:?}",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0021 need more tokens"));
        }

        // 서브쿼리 파싱
        let select = self.handle_select_query(context)?;

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0022 need more tokens"));
        }

        // ) 삼킴
        let current_token = self.get_next_token();

        if current_token != Token::RightParentheses {
            return Err(ParsingError::boxed(format!(
                "E0023 expected right parentheses. but your input is {:?}",
                current_token
            )));
        }

        Ok(select.into())
    }
}
