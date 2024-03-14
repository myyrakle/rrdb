use crate::errors::predule::LexingError;
use crate::lexer::predule::{OperatorToken, Token};
use crate::logger::predule::Logger;
use std::error::Error;

#[derive(Debug)]
pub struct Tokenizer {
    buffer: Vec<char>,
    buffer_index: usize,
    last_char: char,
}

impl Tokenizer {
    pub fn new(text: String) -> Self {
        Logger::info(format!("SQL echo: {:?}", text));
        Self {
            last_char: ' ',
            buffer: text.chars().collect(),
            buffer_index: 0,
        }
    }

    pub fn is_whitespace(&self) -> bool {
        self.last_char == ' ' || self.last_char == '\n' || self.last_char == '\t'
    }

    pub fn is_digit(&self) -> bool {
        self.last_char.is_ascii_digit()
    }

    pub fn is_alphabet(&self) -> bool {
        self.last_char.is_alphabetic()
    }

    pub fn is_alphabet_or_number(&self) -> bool {
        self.last_char.is_alphanumeric()
    }

    pub fn is_underscore(&self) -> bool {
        self.last_char == '_'
    }

    pub fn is_backslash(&self) -> bool {
        self.last_char == '\\'
    }

    pub fn is_special_character(&self) -> bool {
        ['+', '-', '*', '/', ',', '>', '<', '=', '!', '\\'].contains(&self.last_char)
    }

    pub fn is_quote(&self) -> bool {
        ['\'', '"'].contains(&self.last_char)
    }

    pub fn is_semicolon(&self) -> bool {
        self.last_char == ';'
    }

    pub fn is_dot(&self) -> bool {
        self.last_char == '.'
    }

    pub fn is_backtick(&self) -> bool {
        self.last_char == '`'
    }

    pub fn is_parentheses(&self) -> bool {
        self.last_char == '(' || self.last_char == ')'
    }

    pub fn is_eof(&self) -> bool {
        self.buffer_index >= self.buffer.len()
    }

    // 버퍼에서 문자 하나를 읽어서 last_char에 보관합니다.
    pub fn read_char(&mut self) {
        if self.buffer_index >= self.buffer.len() {
            self.last_char = ' ';
        } else {
            self.last_char = self.buffer[self.buffer_index];
            self.buffer_index += 1;
        }
    }

    // 보관했던 문자 하나를 다시 버퍼에 돌려놓습니다.
    pub fn unread_char(&mut self) {
        if self.buffer_index == 0 {
            self.last_char = ' ';
        } else {
            self.buffer_index -= 1;
            self.last_char = self.buffer[self.buffer_index];
        }
    }

    // 주어진 텍스트에서 토큰을 순서대로 획득해 반환합니다.
    // 끝을 만날 경우 Token::EOF를 반환합니다.
    pub fn get_token(&mut self) -> Result<Token, Box<dyn Error + Send>> {
        // 화이트 스페이스 삼킴
        while self.is_whitespace() && !self.is_eof() {
            self.read_char();
        }

        // 첫번째 글짜가 알파벳일 경우 식별자 및 키워드로 인식
        let token = if self.is_alphabet() || self.is_underscore() {
            let mut identifier = vec![self.last_char];

            self.read_char();
            while self.is_alphabet_or_number() || self.is_underscore() {
                identifier.push(self.last_char);
                self.read_char();
            }

            let identifier: String = identifier.into_iter().collect::<String>();

            let token = match identifier.to_uppercase().as_str() {
                "SELECT" => Token::Select,
                "FROM" => Token::From,
                "WHERE" => Token::Where,
                "AS" => Token::As,
                "ORDER" => Token::Order,
                "BY" => Token::By,
                "ASC" => Token::Asc,
                "DESC" => Token::Desc,
                "GROUP" => Token::Group,
                "HAVING" => Token::Having,
                "LIMIT" => Token::Limit,
                "OFFSET" => Token::Offset,
                "INSERT" => Token::Insert,
                "INTO" => Token::Into,
                "VALUES" => Token::Values,
                "UPDATE" => Token::Update,
                "SET" => Token::Set,
                "DELETE" => Token::Delete,
                "JOIN" => Token::Join,
                "INNER" => Token::Inner,
                "LEFT" => Token::Left,
                "RIGHT" => Token::Right,
                "FULL" => Token::Full,
                "OUTER" => Token::Outer,
                "CREATE" => Token::Create,
                "ALTER" => Token::Alter,
                "DROP" => Token::Drop,
                "DATABASE" => Token::Database,
                "TABLE" => Token::Table,
                "COLUMN" => Token::Column,
                "COMMENT" => Token::Comment,
                "PRIMARY" => Token::Primary,
                "FOREIGN" => Token::Foreign,
                "KEY" => Token::Key,
                "ADD" => Token::Add,
                "RENAME" => Token::Rename,
                "TO" => Token::To,
                "SHOW" => Token::Show,
                "DATABASES" => Token::Databases,
                "TABLES" => Token::Tables,
                "AND" => Token::And,
                "OR" => Token::Or,
                "NOT" => Token::Not,
                "BETWEEN" => Token::Between,
                "LIKE" => Token::Like,
                "IN" => Token::In,
                "IS" => Token::Is,
                "TRUE" => Token::Boolean(true),
                "FALSE" => Token::Boolean(false),
                "NULL" => Token::Null,
                "DEFAULT" => Token::Default,
                "IF" => Token::If,
                "EXISTS" => Token::Exists,
                "ON" => Token::On,
                "USE" => Token::Use,
                "DATA" => Token::Data,
                "TYPE" => Token::Type,
                "NULLS" => Token::Nulls,
                "FIRST" => Token::First,
                "LAST" => Token::Last,
                _ => Token::Identifier(identifier),
            };

            return Ok(token);
        }
        // 첫번째 글자가 숫자일 경우 정수 및 실수값으로 인식
        else if self.is_digit() {
            let mut number_string = vec![self.last_char];

            // 숫자나 .이 나올 때까지만 버퍼에서 읽어서 number_string에 저장
            loop {
                self.read_char();
                if self.is_digit() || self.is_dot() {
                    number_string.push(self.last_char);
                    continue;
                } else if self.is_eof() {
                    break;
                } else {
                    self.unread_char();
                    break;
                }
            }

            let number_string: String =
                number_string.into_iter().collect::<String>().to_uppercase();

            // .이 있을 경우 실수, 아닌 경우 정수로 인식
            if number_string.contains('.') {
                let number = number_string.parse::<f64>();

                match number {
                    Ok(number) => Token::Float(number),
                    Err(_) => {
                        return Err(LexingError::boxed(format!(
                            "invalid floating point number format: {}",
                            number_string
                        )))
                    }
                }
            } else {
                let number = number_string.parse::<i64>();

                match number {
                    Ok(number) => Token::Integer(number),
                    Err(_) => {
                        return Err(LexingError::boxed(format!(
                            "invalid integer number format: {}",
                            number_string
                        )))
                    }
                }
            }
        }
        // 특수문자일 경우
        else if self.is_special_character() {
            match self.last_char {
                ',' => Token::Comma,
                '\\' => Token::Backslash,
                '-' => {
                    // 다음 문자가 또 -일 경우 행 단위 주석으로 처리
                    self.read_char();

                    if self.last_char == '-' {
                        let mut comment = vec![];

                        while !self.is_eof() {
                            self.read_char();

                            if self.last_char == '\n' {
                                break;
                            } else {
                                comment.push(self.last_char);
                            }
                        }

                        let comment: String = comment.into_iter().collect();
                        Token::CodeComment(comment)
                    } else {
                        self.unread_char();
                        Token::Operator(OperatorToken::Minus)
                    }
                }
                '/' => {
                    // 다음 문자가 *일 경우 블록 단위 주석으로 처리

                    self.read_char();

                    if self.last_char == '*' {
                        let mut comment = vec![];

                        self.read_char();
                        while !self.is_eof() {
                            if self.last_char == '*' {
                                self.read_char();
                                if self.last_char == '/' {
                                    break;
                                }
                            } else {
                                comment.push(self.last_char);
                            }

                            self.read_char();
                        }

                        let comment: String = comment.into_iter().collect();
                        Token::CodeComment(comment)
                    } else {
                        self.unread_char();
                        Token::Operator(OperatorToken::Slash)
                    }
                }
                '+' => Token::Operator(OperatorToken::Plus),
                '*' => Token::Operator(OperatorToken::Asterisk),
                '!' => Token::Operator(OperatorToken::Not), // TODO: != 연산자 처리
                '=' => Token::Operator(OperatorToken::Eq),
                '<' => Token::Operator(OperatorToken::Lt), // TODO: <= 연산자 처리
                '>' => Token::Operator(OperatorToken::Gt), // TODO: >= 연산자 처리
                _ => {
                    return Err(LexingError::boxed(format!(
                        "unexpected operator: {:?}",
                        self.last_char
                    )))
                }
            }
        }
        // 따옴표일 경우 처리
        else if self.is_quote() {
            if self.last_char == '"' {
                let mut identifier = vec![];

                self.read_char();
                while self.last_char != '"' {
                    identifier.push(self.last_char);
                    self.read_char();
                }

                let identifier: String = identifier.into_iter().collect::<String>();

                Token::Identifier(identifier)
            } else if self.last_char == '\'' {
                let mut string = vec![];

                self.read_char();
                while !self.is_eof() {
                    if self.last_char == '\'' {
                        self.read_char();

                        // '' 의 형태일 경우 '로 이스케이프
                        // 아닐 경우 문자열 종료
                        if self.last_char == '\'' {
                            string.push(self.last_char);
                        } else {
                            self.unread_char();
                            break;
                        }
                    } else {
                        string.push(self.last_char);
                    }

                    self.read_char();
                }

                let string: String = string.into_iter().collect::<String>();

                Token::String(string)
            } else {
                Token::UnknownCharacter(self.last_char)
            }
        } else if self.is_backtick() {
            let mut string = vec![];

            self.read_char();
            while !self.is_eof() {
                if self.last_char == '`' {
                    self.read_char();

                    // `` 의 형태일 경우 `로 이스케이프
                    // 아닐 경우 문자열 종료
                    if self.last_char == '`' {
                        string.push(self.last_char);
                    } else {
                        self.unread_char();
                        break;
                    }
                } else {
                    string.push(self.last_char);
                }

                self.read_char();
            }

            let string: String = string.into_iter().collect::<String>();

            Token::Identifier(string)
        }
        // 세미콜론
        else if self.is_semicolon() {
            Token::SemiColon
        }
        // 마침표
        else if self.is_dot() {
            Token::Period
        }
        // 괄호
        else if self.is_parentheses() {
            if self.last_char == '(' {
                Token::LeftParentheses
            } else {
                Token::RightParentheses
            }
        }
        // 아무것도 해당되지 않을 경우 예외처리
        else if self.is_eof() {
            Token::EOF
        } else {
            return Err(LexingError::boxed(format!(
                "unexpected character: {:?}",
                self.last_char
            )));
        };

        self.last_char = ' ';

        Ok(token)
    }

    // Tokenizer 생성 없이 토큰 목록을 가져올 수 있는 유틸 함수입니다.
    pub fn string_to_tokens(text: String) -> Result<Vec<Token>, Box<dyn Error + Send>> {
        let mut tokenizer = Tokenizer::new(text);

        let mut tokens = vec![];

        while !tokenizer.is_eof() {
            tokens.push(tokenizer.get_token()?);
        }

        Ok(tokens)
    }
}

impl std::fmt::Display for Tokenizer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "Tokenizer: {:?}", self)
    }
}
