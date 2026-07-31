#![cfg(test)]
//! Regression tests for lookahead at end-of-input (found by the property
//! tests added for issue #222).

use crate::engine::lexer::predule::{OperatorToken, Token, Tokenizer};

/// Operators whose tokenizer branch peeks at the next character to decide
/// between a one- and two-character token (`--`, `//`, `<=`, `>=`, `!=`).
/// When such an operator is the final character of the input, the lookahead
/// hits EOF, and the paired `unread_char` used to rewind over a character
/// that was never consumed — so the tokenizer emitted the same operator
/// forever and never reached EOF.
#[test]
fn trailing_lookahead_operator_terminates() {
    let cases = [
        ("-", OperatorToken::Minus),
        ("/", OperatorToken::Slash),
        ("<", OperatorToken::Lt),
        (">", OperatorToken::Gt),
        ("!", OperatorToken::Not),
    ];

    for (input, expected) in cases {
        let mut tokenizer = Tokenizer::new(input.to_owned());

        assert_eq!(
            tokenizer.get_token().unwrap(),
            Token::Operator(expected),
            "first token for {input:?}"
        );
        assert_eq!(
            tokenizer.get_token().unwrap(),
            Token::EOF,
            "{input:?} must reach EOF instead of repeating the operator"
        );
    }
}

/// The same operators must still combine with a following character.
#[test]
fn lookahead_operator_still_combines() {
    let cases = [
        ("<=", Token::Operator(OperatorToken::Lte)),
        (">=", Token::Operator(OperatorToken::Gte)),
        ("!=", Token::Operator(OperatorToken::Neq)),
    ];

    for (input, expected) in cases {
        let mut tokenizer = Tokenizer::new(input.to_owned());
        assert_eq!(tokenizer.get_token().unwrap(), expected, "input {input:?}");
        assert_eq!(tokenizer.get_token().unwrap(), Token::EOF);
    }
}

/// `\r` is whitespace. Without this, any statement carrying CRLF line endings
/// — a script saved on Windows, or a client that sends CRLF — failed to
/// tokenize with `unexpected character: '\r'`.
#[test]
fn carriage_return_is_whitespace() {
    let mut tokenizer = Tokenizer::new("\r\n".to_owned());
    assert_eq!(tokenizer.get_token().unwrap(), Token::EOF);

    let tokens = Tokenizer::string_to_tokens("SELECT\r\n1".to_owned()).unwrap();
    assert!(
        tokens.iter().any(|token| matches!(token, Token::Select)),
        "CRLF-separated SQL should tokenize, got {tokens:?}"
    );
}

/// A quoted identifier with no closing quote used to spin forever: the loop
/// scanning for the closing `"` had no EOF guard, so it read past the end of
/// the buffer indefinitely. It must terminate with an error instead.
#[test]
fn unterminated_quoted_identifier_errors() {
    assert!(Tokenizer::string_to_tokens("\"".to_owned()).is_err());
    assert!(Tokenizer::string_to_tokens("SELECT \"col".to_owned()).is_err());

    // A properly closed quoted identifier still works.
    let tokens = Tokenizer::string_to_tokens("\"col\"".to_owned()).unwrap();
    assert_eq!(tokens, vec![Token::Identifier("col".to_owned())]);
}
