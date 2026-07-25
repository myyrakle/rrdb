#![cfg(test)]
//! Property-based tests for the lexer/parser (issue #222, phase 1-2).
//!
//! Example-based tests can only cover inputs the author thought of. These
//! generate thousands of inputs per run and assert invariants that must hold
//! for *every* input, which is where malformed-SQL handling tends to break.

use proptest::prelude::*;

use crate::engine::lexer::predule::Tokenizer;
use crate::engine::parser::context::ParserContext;
use crate::engine::parser::predule::Parser;

/// Parse a statement the way the engine does: tokenize, then parse.
fn try_parse(sql: &str) -> crate::errors::Result<Vec<crate::engine::ast::SQLStatement>> {
    let tokens = Tokenizer::string_to_tokens(sql.to_owned())?;
    Parser::new(tokens).parse(ParserContext::default())
}

/// Arbitrary text, including control characters, quotes and unbalanced
/// delimiters.
fn arbitrary_sql_text() -> impl Strategy<Value = String> {
    proptest::string::string_regex(r#"[a-zA-Z0-9_ ,;'"()\-*=<>\.\r\n\t]{0,80}"#)
        .expect("valid regex")
}

/// Fragments drawn from real SQL keywords, so the generator spends more of its
/// budget on inputs that reach deeper into the parser instead of being
/// rejected by the tokenizer immediately.
fn sql_like_fragment() -> impl Strategy<Value = String> {
    let keyword = prop::sample::select(vec![
        "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE",
        "TABLE", "DATABASE", "INDEX", "DROP", "ALTER", "AND", "OR", "NOT", "NULL", "PRIMARY",
        "KEY", "ORDER", "BY", "GROUP", "JOIN", "ON", "AS", "(", ")", ",", ";", "*", "=", "'x'",
        "1", "foo",
    ]);

    prop::collection::vec(keyword, 0..12).prop_map(|parts| parts.join(" "))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1024))]

    /// The parser must never panic, however malformed the input. Any input it
    /// cannot handle has to come back as an `Err`, not an abort — a panic here
    /// would take down the connection task that is parsing the statement.
    #[test]
    fn parser_never_panics_on_arbitrary_text(sql in arbitrary_sql_text()) {
        let _ = try_parse(&sql);
    }

    /// Same invariant, but with inputs built from real SQL keywords so the
    /// generator reaches the statement parsers rather than failing at the
    /// tokenizer.
    #[test]
    fn parser_never_panics_on_sql_like_input(sql in sql_like_fragment()) {
        let _ = try_parse(&sql);
    }

    /// Parsing is a pure function of its input: the same text must always
    /// produce the same outcome. This guards against parser state leaking
    /// across runs through shared or cached state.
    #[test]
    fn parsing_is_deterministic(sql in sql_like_fragment()) {
        let first = try_parse(&sql);
        let second = try_parse(&sql);

        prop_assert_eq!(first.is_ok(), second.is_ok());

        if let (Ok(first), Ok(second)) = (first, second) {
            prop_assert_eq!(format!("{:?}", first), format!("{:?}", second));
        }
    }

    /// Leading and trailing whitespace must not change the parse result.
    #[test]
    fn surrounding_whitespace_is_insignificant(sql in sql_like_fragment()) {
        let bare = try_parse(&sql);
        let padded = try_parse(&format!("  \t{sql}\n "));

        prop_assert_eq!(bare.is_ok(), padded.is_ok());

        if let (Ok(bare), Ok(padded)) = (bare, padded) {
            prop_assert_eq!(format!("{:?}", bare), format!("{:?}", padded));
        }
    }

    /// The tokenizer must terminate and must not invent trailing tokens for
    /// input that is only whitespace.
    #[test]
    fn whitespace_only_input_yields_no_statements(
        spaces in proptest::string::string_regex(r"[ \t\r\n]{0,40}").expect("valid regex")
    ) {
        let parsed = try_parse(&spaces);
        prop_assert!(parsed.is_ok(), "whitespace should parse, got {:?}", parsed.err());
        prop_assert!(parsed.unwrap().is_empty());
    }
}
