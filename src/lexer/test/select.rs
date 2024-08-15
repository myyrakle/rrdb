#[cfg(test)]
use crate::lexer::predule::OperatorToken;
#[cfg(test)]
use crate::lexer::predule::{Token, Tokenizer};

#[test]
pub fn test_number_literal() {
    struct TestCase {
        name: String,
        input: String,
        want_error: bool,
        expected: Vec<Token>,
    }

    let test_cases = vec![
        TestCase {
            name: "한자리수 정수".to_owned(),
            input: "SELECT 1".to_owned(),
            want_error: false,
            expected: vec![Token::Select, Token::Integer(1)],
        },
        TestCase {
            name: "여러자리 정수".to_owned(),
            input: "SELECT 1432".to_owned(),
            want_error: false,
            expected: vec![Token::Select, Token::Integer(1432)],
        },
        TestCase {
            name: "정수 파싱 실패".to_owned(),
            input: "SELECT 1444444444444444444444444444444444444444444444444444444432".to_owned(),
            want_error: true,
            expected: vec![],
        },
        TestCase {
            name: "실수: 3.14".to_owned(),
            input: "SELECT 3.14".to_owned(),
            want_error: false,
            expected: vec![Token::Select, Token::Float(3.14)],
        },
        TestCase {
            name: "실수 파싱 실패: 3..14".to_owned(),
            input: "SELECT 3..14".to_owned(),
            want_error: true,
            expected: vec![],
        },
    ];

    for t in test_cases {
        let got = Tokenizer::string_to_tokens(t.input);

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(tokens) = got {
            assert_eq!(tokens, t.expected, "{}", t.name);
        }
    }
}

#[test]
pub fn select_text() {
    struct TestCase {
        name: String,
        input: String,
        want_error: bool,
        expected: Vec<Token>,
    }

    let test_cases = vec![
        TestCase {
            name: "문자열: 'I''m Sam'".to_owned(),
            input: r#"SELECT 'I''m Sam'"#.to_owned(),
            want_error: false,
            expected: vec![Token::Select, Token::String("I'm Sam".to_owned())],
        },
        TestCase {
            name: "빈 문자열".to_owned(),
            input: r#"SELECT ''"#.to_owned(),
            want_error: false,
            expected: vec![Token::Select, Token::String("".to_owned())],
        },
    ];

    for t in test_cases {
        let got = Tokenizer::string_to_tokens(t.input);

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(tokens) = got {
            assert_eq!(tokens, t.expected, "{}", t.name);
        }
    }
}

#[test]
pub fn test_errors() {
    struct TestCase {
        name: String,
        input: String,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "예상하지 못한 특수문자".to_owned(),
            input: r#"SELECT @"#.to_owned(),
            want_error: true,
        },
        // TestCase {
        //     name: "예상하지 못한 특수문자: $".to_owned(),
        //     input: r#"SELECT $"#.to_owned(),
        //     want_error: true,
        // },
        TestCase {
            name: "예상하지 못한 특수문자: $$$".to_owned(),
            input: r#"SELECT $$$"#.to_owned(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let got = Tokenizer::string_to_tokens(t.input);

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );
    }
}

#[test]
pub fn test_operators() {
    struct TestCase {
        name: String,
        input: String,
        want_error: bool,
        expected: Vec<Token>,
    }

    let test_cases = vec![
        TestCase {
            name: "연산자: /".to_owned(),
            input: r#"SELECT 1 / 2"#.to_owned(),
            want_error: false,
            expected: vec![
                Token::Select,
                Token::Integer(1),
                Token::Operator(OperatorToken::Slash),
                Token::Integer(2),
            ],
        },
        TestCase {
            name: "연산자: <".to_owned(),
            input: r#"SELECT 1 < 2"#.to_owned(),
            want_error: false,
            expected: vec![
                Token::Select,
                Token::Integer(1),
                Token::Operator(OperatorToken::Lt),
                Token::Integer(2),
            ],
        },
        TestCase {
            name: "연산자: >".to_owned(),
            input: r#"SELECT 1 > 2"#.to_owned(),
            want_error: false,
            expected: vec![
                Token::Select,
                Token::Integer(1),
                Token::Operator(OperatorToken::Gt),
                Token::Integer(2),
            ],
        },
        TestCase {
            name: "연산자: -".to_owned(),
            input: r#"SELECT 1 - 2"#.to_owned(),
            want_error: false,
            expected: vec![
                Token::Select,
                Token::Integer(1),
                Token::Operator(OperatorToken::Minus),
                Token::Integer(2),
            ],
        },
        TestCase {
            name: "연산자: +".to_owned(),
            input: r#"SELECT 1 + 2"#.to_owned(),
            want_error: false,
            expected: vec![
                Token::Select,
                Token::Integer(1),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(2),
            ],
        },
        TestCase {
            name: "연산자: *".to_owned(),
            input: r#"SELECT 1 * 2"#.to_owned(),
            want_error: false,
            expected: vec![
                Token::Select,
                Token::Integer(1),
                Token::Operator(OperatorToken::Asterisk),
                Token::Integer(2),
            ],
        },
        TestCase {
            name: "연산자: =".to_owned(),
            input: r#"SELECT 1 = 2"#.to_owned(),
            want_error: false,
            expected: vec![
                Token::Select,
                Token::Integer(1),
                Token::Operator(OperatorToken::Eq),
                Token::Integer(2),
            ],
        },
        TestCase {
            name: "연산자: !".to_owned(),
            input: r#"SELECT ! True"#.to_owned(),
            want_error: false,
            expected: vec![
                Token::Select,
                Token::Operator(OperatorToken::Not),
                Token::Boolean(true),
            ],
        },
    ];

    for t in test_cases {
        let got = Tokenizer::string_to_tokens(t.input);

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(tokens) = got {
            assert_eq!(tokens, t.expected, "{}", t.name);
        }
    }
}

#[test]
pub fn test_identifier() {
    struct TestCase {
        name: String,
        input: String,
        want_error: bool,
        expected: Vec<Token>,
    }

    let test_cases = vec![
        TestCase {
            name: "백틱 파싱".to_owned(),
            input: r#"SELECT `foo`"#.to_owned(),
            want_error: false,
            expected: vec![Token::Select, Token::Identifier("foo".to_owned())],
        },
        TestCase {
            name: "백틱 안에 백틱 파싱".to_owned(),
            input: r#"SELECT `foo``bar`"#.to_owned(),
            want_error: false,
            expected: vec![Token::Select, Token::Identifier("foo`bar".to_owned())],
        },
    ];

    for t in test_cases {
        let got = Tokenizer::string_to_tokens(t.input);

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(tokens) = got {
            assert_eq!(tokens, t.expected, "{}", t.name);
        }
    }
}

#[test]
pub fn select_from_1() {
    let text = r#"SELECT name from person"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Select,
            Token::Identifier("name".to_owned()),
            Token::From,
            Token::Identifier("person".to_owned())
        ]
    );
}

#[test]
pub fn select_from_2() {
    let text = r#"SELECT 1 from "boom""#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Select,
            Token::Integer(1),
            Token::From,
            Token::Identifier("boom".to_owned())
        ]
    );
}

#[test]
pub fn select_from_where_1() {
    let text = r#"SELECT name from person where"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Select,
            Token::Identifier("name".to_owned()),
            Token::From,
            Token::Identifier("person".to_owned()),
            Token::Where,
        ]
    );
}

// #[test]
// pub fn inner_join() {
//     let text = r#"
//         SELECT
//             p.name as name,
//             s.name as schoolName
//         from person as p
//         inner join school as s
//         on 1=1
//             and p.school_id = s.id
//     "#
//     .to_owned();

//     let tokens = Tokenizer::string_to_tokens(text);

//     assert_eq!(
//         tokens,
//         vec![
//             Token::Select,
//             Token::Identifier("p.name".to_owned()),
//             Token::As,
//             Token::Identifier("name".to_owned()),
//             Token::From,
//             Token::Identifier("person".to_owned()),
//             Token::Where,
//         ]
//     );
// }

#[test]
fn test_command_query() {
    let text = r#"\l"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(
        tokens,
        vec![Token::Backslash, Token::Identifier("l".to_owned()),]
    );
}
