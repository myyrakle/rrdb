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
///
/// The leading keyword is chosen separately from the rest. A uniform shuffle
/// of keywords almost never starts with one that opens a statement, so the
/// parser returns an empty statement list and the deeper code is never
/// entered: measured over 1024 generated inputs, exactly 0-1 produced a
/// statement. Pinning the first token keeps the mutations where they are
/// useful -- inside a statement the parser has actually committed to.
fn sql_like_fragment() -> impl Strategy<Value = String> {
    let leading = prop::sample::select(vec![
        "SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER",
    ]);
    let keyword = prop::sample::select(vec![
        "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE",
        "TABLE", "DATABASE", "INDEX", "DROP", "ALTER", "AND", "OR", "NOT", "NULL", "PRIMARY",
        "KEY", "ORDER", "BY", "GROUP", "JOIN", "ON", "AS", "(", ")", ",", ";", "*", "=", "'x'",
        "1", "foo",
    ]);

    (leading, prop::collection::vec(keyword, 0..12)).prop_map(|(head, rest)| {
        let mut parts = Vec::with_capacity(rest.len() + 1);
        parts.push(head);
        parts.extend(rest);
        parts.join(" ")
    })
}

/// Well-formed statements with one mutation applied. The fragment generators
/// above explore malformed input, which is where panics hide, but they almost
/// never produce something the parser accepts -- so on their own they never
/// exercise the code that runs *after* a statement is recognised. These start
/// from a valid statement and perturb it, so the parser commits to a statement
/// kind first and then meets the unexpected token.
fn mutated_statement() -> impl Strategy<Value = String> {
    let base = prop::sample::select(vec![
        "SELECT 1",
        "SELECT * FROM foo",
        "SELECT foo, bar FROM baz WHERE foo = 1",
        "INSERT INTO foo (a, b) VALUES (1, 'x')",
        "UPDATE foo SET a = 1 WHERE b = 'x'",
        "DELETE FROM foo WHERE a = 1",
        "CREATE TABLE foo (a INTEGER PRIMARY KEY)",
        "DROP TABLE foo",
    ]);
    let injection = prop::sample::select(vec![
        "", " ", ",", "(", ")", "'", ";", "*", "=", "NULL", "SELECT", "WHERE", "\n", "\t",
    ]);

    (base, injection, 0usize..40).prop_map(|(base, injection, at)| {
        let at = at.min(base.len());
        let mut out = String::with_capacity(base.len() + injection.len());
        out.push_str(&base[..base.floor_char_boundary(at)]);
        out.push_str(injection);
        out.push_str(&base[base.floor_char_boundary(at)..]);
        out
    })
}

/// Statements that are valid by construction. Needed because every other
/// generator here explores *malformed* input: they prove the parser does not
/// panic, but nothing so far asserts that well-formed SQL actually parses. A
/// parser that rejected everything would satisfy all the panic and determinism
/// properties below.
fn valid_statement() -> impl Strategy<Value = String> {
    let table = prop::sample::select(vec!["foo", "bar", "baz_1"]);
    let column = prop::sample::select(vec!["a", "b", "id"]);
    let literal = prop::sample::select(vec!["1", "42", "'x'"]);
    let terminator = prop::sample::select(vec!["", ";"]);

    (table, column, literal, terminator).prop_flat_map(
        |(table, column, literal, terminator)| {
            let shapes = vec![
                format!("SELECT {} FROM {}", column, table),
                format!("SELECT * FROM {}", table),
                format!("SELECT {} FROM {} WHERE {} = {}", column, table, column, literal),
                format!("INSERT INTO {} ({}) VALUES ({})", table, column, literal),
                format!("UPDATE {} SET {} = {}", table, column, literal),
                format!("DELETE FROM {} WHERE {} = {}", table, column, literal),
                format!("DROP TABLE {}", table),
            ];
            prop::sample::select(shapes).prop_map(move |shape| format!("{}{}", shape, terminator))
        },
    )
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

    /// The same invariant on inputs the parser actually accepts far more often:
    /// a valid statement with one token injected. Measured over 1024 generated
    /// inputs, `sql_like_fragment` yields 0-1 parsed statements while this
    /// yields ~480, so this is the generator that reaches the code paths
    /// running after a statement kind has been recognised.
    #[test]
    fn parser_never_panics_on_mutated_statement(sql in mutated_statement()) {
        let _ = try_parse(&sql);
    }

    /// Parsing a mutated statement is deterministic too. Kept separate from
    /// the fragment version because this one exercises the statement bodies.
    #[test]
    fn parsing_a_mutated_statement_is_deterministic(sql in mutated_statement()) {
        prop_assert_eq!(try_parse(&sql).is_ok(), try_parse(&sql).is_ok());
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
    /// The success half of the contract: well-formed SQL must parse, and must
    /// produce exactly one statement. Without this, "the parser never panics"
    /// would also hold for a parser that rejected every input.
    #[test]
    fn valid_statements_parse_into_exactly_one_statement(sql in valid_statement()) {
        let parsed = try_parse(&sql);
        prop_assert!(
            parsed.is_ok(),
            "valid SQL should parse: {:?} -> {:?}",
            sql,
            parsed.as_ref().err()
        );
        prop_assert_eq!(
            parsed.unwrap().len(),
            1,
            "valid SQL should yield one statement: {:?}",
            sql
        );
    }

    /// The failure half: input that cannot be a statement must come back as an
    /// error, not as a silently empty parse. A statement keyword with nothing
    /// after it is unambiguously incomplete.
    #[test]
    fn a_bare_statement_keyword_is_an_error(
        keyword in prop::sample::select(vec!["SELECT", "INSERT", "UPDATE", "DELETE", "DROP"])
    ) {
        let parsed = try_parse(keyword);
        prop_assert!(
            parsed.is_err(),
            "{:?} alone is incomplete and must be rejected, got {:?}",
            keyword,
            parsed.ok()
        );
    }

    #[test]
    fn whitespace_only_input_yields_no_statements(
        spaces in proptest::string::string_regex(r"[ \t\r\n]{0,40}").expect("valid regex")
    ) {
        let parsed = try_parse(&spaces);
        prop_assert!(parsed.is_ok(), "whitespace should parse, got {:?}", parsed.err());
        prop_assert!(parsed.unwrap().is_empty());
    }
}


