use super::Token;

pub struct Tokenizer {
    buffer: Vec<char>,
    buffer_index: usize,
    last_char: char,
}

impl Tokenizer {
    pub fn new(text: String) -> Self {
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
        self.last_char.is_digit(10)
    }

    pub fn is_alphabet(&self) -> bool {
        self.last_char.is_alphabetic()
    }

    pub fn is_alphabet_or_number(&self) -> bool {
        self.last_char.is_alphanumeric()
    }

    pub fn is_dot(&self) -> bool {
        self.last_char == '.'
    }

    pub fn is_backtic(&self) -> bool {
        self.last_char == '`'
    }

    pub fn is_eof(&self) -> bool {
        self.buffer_index >= self.buffer.len()
    }

    pub fn read_char(&mut self) {
        if self.buffer_index >= self.buffer.len() {
            self.last_char = ' ';
        } else {
            self.last_char = self.buffer[self.buffer_index];
            self.buffer_index += 1;
        }
    }

    // 주어진 텍스트에서 토큰을 순서대로 획득해 반환합니다.
    // 끝을 만날 경우 Token::EOF를 반환합니다.
    pub fn get_token(&mut self) -> Token {
        // 화이트 스페이스 삼킴
        while self.is_whitespace() {
            self.read_char();
        }

        // 첫번째 글짜가 알파벳일 경우 식별자 및 키워드로 인식
        if self.is_alphabet() {
            let mut identifier = vec![self.last_char];

            self.read_char();
            while self.is_alphabet_or_number() {
                identifier.push(self.last_char);
                self.read_char();
            }

            let identifier: String = identifier.into_iter().collect::<String>();

            return match identifier.to_uppercase().as_str() {
                "SELECT" => Token::Select,
                "FROM" => Token::From,
                "WHERE" => Token::Where,
                _ => Token::Identifier(identifier),
            };
        }
        // 첫번째 글자가 숫자일 경우 정수 및 실수값으로 인식
        else if self.is_digit() {
            let mut number_string = vec![self.last_char];

            self.read_char();
            while self.is_digit() || self.is_dot() {
                number_string.push(self.last_char);
                self.read_char();
            }

            let number_string: String =
                number_string.into_iter().collect::<String>().to_uppercase();

            // .이 있을 경우 실수, 아닌 경우 정수로 인식
            if number_string.contains('.') {
                let number = number_string.parse::<f64>();

                match number {
                    Ok(number) => Token::Float(number),
                    Err(_) => Token::Error(
                        format!("invalid floating point number format: {}", number_string).into(),
                    ),
                }
            } else {
                let number = number_string.parse::<i64>();

                match number {
                    Ok(number) => Token::Integer(number),
                    Err(_) => Token::Error(
                        format!("invalid integer number format: {}", number_string).into(),
                    ),
                }
            }
        }
        // 아무것도 해당되지 않을 경우 예외처리
        else if self.is_eof() {
            Token::EOF
        } else {
            Token::UnknownCharacter(self.last_char)
        }
    }

    // Tokenizer 생성 없이 토큰 목록을 가져올 수 있는 유틸 함수입니다.
    pub fn string_to_tokens(text: String) -> Vec<Token> {
        let mut tokenizer = Tokenizer::new(text);

        let mut tokens = vec![];

        while !tokenizer.is_eof() {
            tokens.push(tokenizer.get_token());
        }

        tokens
    }
}
