//! 입력 끝(EOF) 경계에서의 토크나이저 동작 회귀 테스트.
//!
//! 관련 이슈: #237
#[cfg(test)]
use crate::engine::lexer::predule::{OperatorToken, Token, Tokenizer};

#[test]
pub fn test_token_before_eof_is_not_dropped() {
    struct TestCase {
        name: String,
        input: String,
        expected: Vec<Token>,
    }

    let test_cases = vec![
        TestCase {
            name: "세미콜론 없이 닫는 괄호로 끝나는 입력".to_owned(),
            input: "create index foo_id_idx on foo (id)".to_owned(),
            expected: vec![
                Token::Create,
                Token::Index,
                Token::Identifier("foo_id_idx".to_owned()),
                Token::On,
                Token::Identifier("foo".to_owned()),
                Token::LeftParentheses,
                Token::Identifier("id".to_owned()),
                Token::RightParentheses,
            ],
        },
        TestCase {
            name: "닫는 괄호로 끝나는 표현식".to_owned(),
            input: "select (1)".to_owned(),
            expected: vec![
                Token::Select,
                Token::LeftParentheses,
                Token::Integer(1),
                Token::RightParentheses,
            ],
        },
        TestCase {
            name: "여는 괄호로 끝나는 입력".to_owned(),
            input: "select (".to_owned(),
            expected: vec![Token::Select, Token::LeftParentheses],
        },
        TestCase {
            name: "세미콜론으로 끝나는 입력".to_owned(),
            input: "select 1;".to_owned(),
            expected: vec![Token::Select, Token::Integer(1), Token::SemiColon],
        },
        TestCase {
            name: "쉼표로 끝나는 입력".to_owned(),
            input: "select 1,".to_owned(),
            expected: vec![Token::Select, Token::Integer(1), Token::Comma],
        },
        TestCase {
            name: "마침표로 끝나는 입력".to_owned(),
            input: "select foo.".to_owned(),
            expected: vec![
                Token::Select,
                Token::Identifier("foo".to_owned()),
                Token::Period,
            ],
        },
        TestCase {
            name: "단항 연산자로 끝나는 입력".to_owned(),
            input: "select 1 <".to_owned(),
            expected: vec![
                Token::Select,
                Token::Integer(1),
                Token::Operator(OperatorToken::Lt),
            ],
        },
        TestCase {
            name: "= 연산자로 끝나는 입력".to_owned(),
            input: "select 1 =".to_owned(),
            expected: vec![
                Token::Select,
                Token::Integer(1),
                Token::Operator(OperatorToken::Eq),
            ],
        },
        TestCase {
            name: "! 연산자로 끝나는 입력".to_owned(),
            input: "select !".to_owned(),
            expected: vec![Token::Select, Token::Operator(OperatorToken::Not)],
        },
        TestCase {
            name: "- 연산자로 끝나는 입력".to_owned(),
            input: "select -".to_owned(),
            expected: vec![Token::Select, Token::Operator(OperatorToken::Minus)],
        },
        TestCase {
            name: "/ 연산자로 끝나는 입력".to_owned(),
            input: "select 4 /".to_owned(),
            expected: vec![
                Token::Select,
                Token::Integer(4),
                Token::Operator(OperatorToken::Slash),
            ],
        },
        TestCase {
            name: "숫자로 끝나는 입력".to_owned(),
            input: "select 42".to_owned(),
            expected: vec![Token::Select, Token::Integer(42)],
        },
        TestCase {
            name: "문자열로 끝나는 입력".to_owned(),
            input: "select 'abc'".to_owned(),
            expected: vec![Token::Select, Token::String("abc".to_owned())],
        },
        TestCase {
            name: "따옴표 식별자로 끝나는 입력".to_owned(),
            input: "select \"abc\"".to_owned(),
            expected: vec![Token::Select, Token::Identifier("abc".to_owned())],
        },
        TestCase {
            name: "백틱 식별자로 끝나는 입력".to_owned(),
            input: "select `abc`".to_owned(),
            expected: vec![Token::Select, Token::Identifier("abc".to_owned())],
        },
        TestCase {
            name: "공백으로 끝나는 입력".to_owned(),
            input: "select (id) ".to_owned(),
            expected: vec![
                Token::Select,
                Token::LeftParentheses,
                Token::Identifier("id".to_owned()),
                Token::RightParentheses,
            ],
        },
    ];

    for t in test_cases {
        let got = Tokenizer::string_to_tokens(t.input.clone());

        assert!(got.is_ok(), "{}: 토큰화 실패: {:?}", t.name, got.err());
        assert_eq!(got.unwrap(), t.expected, "{}", t.name);
    }
}

#[test]
pub fn test_unterminated_quoted_identifier_returns_error() {
    // 닫는 큰따옴표가 없는 입력은 무한루프 대신 오류로 처리되어야 합니다.
    let got = Tokenizer::string_to_tokens("select \"abc".to_owned());

    assert!(got.is_err(), "unterminated quoted identifier: {:?}", got);
}

#[test]
pub fn test_comment_at_eof() {
    // 행 주석이 개행 없이 입력 끝에서 종료되는 경우
    assert_eq!(
        Tokenizer::string_to_tokens("select 1 -- comment".to_owned()).unwrap(),
        vec![
            Token::Select,
            Token::Integer(1),
            Token::CodeComment(" comment".to_owned()),
        ],
    );

    // 행 주석 뒤에 개행과 토큰이 이어지는 경우
    assert_eq!(
        Tokenizer::string_to_tokens("select -- comment\n1".to_owned()).unwrap(),
        vec![
            Token::Select,
            Token::CodeComment(" comment".to_owned()),
            Token::Integer(1),
        ],
    );

    // 블록 주석이 입력 끝에서 닫히는 경우
    assert_eq!(
        Tokenizer::string_to_tokens("select /* comment */".to_owned()).unwrap(),
        vec![Token::Select, Token::CodeComment(" comment ".to_owned())],
    );
}
