use std::error::Error;

use crate::lib::ast::predule::{Column, DataType, TableName};
use crate::lib::errors::predule::ParsingError;
use crate::lib::lexer::predule::{OperatorToken, Token};
use crate::lib::parser::predule::Parser;
use crate::lib::types::SelectColumn;

impl Parser {
    // 테이블 컬럼 정의 분석
    pub(crate) fn parse_table_column(&mut self) -> Result<Column, Box<dyn Error>> {
        let mut builder = Column::builder();

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        if let Token::Identifier(name) = current_token {
            builder = builder.set_name(name);
        } else {
            return Err(ParsingError::boxed(format!(
                "expected identifier. but your input word is '{:?}'",
                current_token
            )));
        }

        let data_type = self.parse_data_type()?;
        builder = builder.set_data_type(data_type);

        loop {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("need more tokens"));
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
                        return Err(ParsingError::boxed("need more tokens"));
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
                        return Err(ParsingError::boxed("need more tokens"));
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
                        return Err(ParsingError::boxed("need more tokens"));
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
    pub(crate) fn parse_data_type(&mut self) -> Result<DataType, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        if let Token::Identifier(type_name) = current_token {
            match type_name.to_uppercase().as_str() {
                "INTEGER" => Ok(DataType::Int),
                "FLOAT" => Ok(DataType::Float),
                "BOOLEAN" => Ok(DataType::Boolean),
                "VARCHAR" => {
                    // 여는 괄호 체크
                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("need more tokens"));
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
                        return Err(ParsingError::boxed("need more tokens"));
                    }

                    let current_token = self.get_next_token();

                    if let Token::Integer(integer) = current_token {
                        // 닫는 괄호 체크
                        if !self.has_next_token() {
                            return Err(ParsingError::boxed("need more tokens"));
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
                        return Err(ParsingError::boxed(format!(
                            "expected integer number. but your input word is '{:?}'",
                            current_token
                        )));
                    }
                }
                _ => Err(ParsingError::boxed(format!(
                    "unknown data type '{}'",
                    type_name
                ))),
            }
        } else {
            return Err(ParsingError::boxed(format!(
                "expected identifier. but your input word is '{:?}'",
                current_token
            )));
        }
    }

    // 테이블명 분석
    pub(crate) fn parse_table_name(&mut self) -> Result<TableName, Box<dyn Error>> {
        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // 첫번째로 오는 이름은 테이블명으로 추정
        let current_token = self.get_next_token();
        let mut table_name;
        let mut database_name = None;

        if let Token::Identifier(name) = current_token {
            table_name = name;
        } else {
            return Err(ParsingError::boxed(format!(
                "expected identifier. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        // .가 있을 경우 "데이터베이스명"."테이블명"의 형태로 추정
        if current_token == Token::Period {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("need more tokens"));
            }

            let current_token = self.get_next_token();

            if let Token::Identifier(name) = current_token {
                database_name = Some(table_name);
                table_name = name;
            } else {
                return Err(ParsingError::boxed(format!(
                    "expected identifier. but your input word is '{:?}'",
                    current_token
                )));
            }
        } else {
            self.unget_next_token(current_token);
        }

        Ok(TableName::new(database_name, table_name))
    }

    // IF NOT EXISTS 체크 로직
    pub(crate) fn has_if_not_exists(&mut self) -> Result<bool, Box<dyn Error>> {
        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::If == current_token {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("need more tokens"));
            }

            let current_token = self.get_next_token();

            if Token::Not == current_token {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("need more tokens"));
                }

                let current_token = self.get_next_token();

                if Token::Exists == current_token {
                    return Ok(true);
                } else {
                    return Err(ParsingError::boxed(format!(
                        "expected keyword is 'exists'. but your input word is '{:?}'",
                        current_token
                    )));
                }
            } else {
                return Err(ParsingError::boxed(format!(
                    "expected keyword is 'not'. but your input word is '{:?}'",
                    current_token
                )));
            }
        } else {
            self.unget_next_token(current_token);
            return Ok(false);
        }
    }

    // IF EXISTS 체크 로직
    pub(crate) fn has_if_exists(&mut self) -> Result<bool, Box<dyn Error>> {
        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::If == current_token {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("need more tokens"));
            }

            let current_token = self.get_next_token();

            if Token::Exists == current_token {
                return Ok(true);
            } else {
                return Err(ParsingError::boxed(format!(
                    "expected keyword is 'exists'. but your input word is '{:?}'",
                    current_token
                )));
            }
        } else {
            self.unget_next_token(current_token);
            return Ok(false);
        }
    }

    // SELECT 컬럼 정의 분석
    pub(crate) fn parse_select_column(&mut self) -> Result<SelectColumn, Box<dyn Error>> {
        let mut select_column = SelectColumn::new(None, "".to_string());

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        if let Token::Identifier(name) = current_token {
            select_column.column_name = name;
        } else {
            return Err(ParsingError::boxed(format!(
                "expected identifier. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Ok(select_column);
        } else {
            let current_token = self.get_next_token();

            if current_token == Token::Period {
                let current_token = self.get_next_token();

                if let Token::Identifier(name) = current_token {
                    select_column.table_name = Some(select_column.column_name);
                    select_column.column_name = name;
                    return Ok(select_column);
                } else {
                    return Err(ParsingError::boxed(format!(
                        "expected identifier. but your input word is '{:?}'",
                        current_token
                    )));
                }
            } else {
                self.unget_next_token(current_token);
                return Ok(select_column);
            }
        }
    }

    // 다음 토큰이 2항 연산자/키워드인지
    pub(crate) fn next_token_is_binary_operator(&mut self) -> bool {
        if !self.has_next_token() {
            return false;
        } else {
            let current_token = self.get_next_token();

            self.unget_next_token(current_token.clone());

            // 2항 키워드, 연산자일 경우에만 true 반환
            match current_token {
                Token::And | Token::Or | Token::Like => return true,
                Token::Operator(operator) => {
                    return [
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
                    .contains(&operator)
                }
                _ => {
                    return false;
                }
            }
        }
    }
}
