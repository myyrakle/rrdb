use std::error::Error;

use crate::lib::parser::Parser;

use crate::lib::{
    Column, CreateTableQuery, DataType, FloatExpression, IExpression, IntegerExpression,
    ParsingError, SQLStatement, Table, Token, Tokenizer,
};

impl Parser {
    // 테이블 컬럼 정의 분석
    pub(crate) fn parse_table_column(&mut self) -> Result<Column, Box<dyn Error>> {
        let mut builder = Column::builder();

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        if let Token::Identifier(name) = current_token {
            builder.set_name(name);
        } else {
            return Err(ParsingError::boxed(format!(
                "expected identifier. but your input word is '{:?}'",
                current_token
            )));
        }

        let data_type = self.parse_data_type()?;
        builder.set_data_type(data_type);

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
                            builder.set_primary_key(true);
                            builder.set_not_null(true);
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
                            builder.set_not_null(true);
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
                    builder.set_not_null(false);
                }
                Token::Comment => {
                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("need more tokens"));
                    }

                    let current_token = self.get_next_token();

                    if let Token::String(comment) = current_token {
                        builder.set_comment(comment);
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
    pub(crate) fn parse_table_name(&mut self) -> Result<Table, Box<dyn Error>> {
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

        Ok(Table::new(database_name, table_name))
    }
}
