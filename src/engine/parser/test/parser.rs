#![cfg(test)]

use std::collections::VecDeque;

use crate::engine::ast::{DMLStatement, OtherStatement, SQLStatement};
use crate::engine::lexer::tokens::Token;
use crate::engine::parser::parser::Parser;
use crate::engine::parser::predule::ParserContext;

#[test]
fn with_tokens_builds_parser_from_a_vecdeque() {
    let tokens: VecDeque<Token> = VecDeque::from(vec![Token::Select]);

    let parser = Parser::with_tokens(tokens);

    assert_eq!(parser.current_token, Token::EOF);
    assert!(parser.has_next_token());
}

#[test]
fn show_tokens_does_not_panic() {
    let parser = Parser::new(vec![Token::Select, Token::EOF]);

    parser.show_tokens();
}

#[test]
fn parse_dispatches_select_statement_at_top_level() {
    let mut parser = Parser::with_string("SELECT * FROM foo;".to_owned()).unwrap();

    let statements = parser.parse(ParserContext::default()).unwrap();

    assert_eq!(statements.len(), 1);
    assert!(matches!(
        statements[0],
        SQLStatement::DML(DMLStatement::SelectQuery(_))
    ));
}

#[test]
fn parse_dispatches_update_statement_at_top_level() {
    let mut parser = Parser::with_string("UPDATE foo SET id = 1;".to_owned()).unwrap();

    let statements = parser.parse(ParserContext::default()).unwrap();

    assert_eq!(statements.len(), 1);
    assert!(matches!(
        statements[0],
        SQLStatement::DML(DMLStatement::UpdateQuery(_))
    ));
}

#[test]
fn parse_dispatches_insert_statement_at_top_level() {
    let mut parser =
        Parser::with_string("INSERT INTO foo (id) SELECT id FROM foo;".to_owned()).unwrap();

    let statements = parser.parse(ParserContext::default()).unwrap();

    assert_eq!(statements.len(), 1);
    assert!(matches!(
        statements[0],
        SQLStatement::DML(DMLStatement::InsertQuery(_))
    ));
}

#[test]
fn parse_dispatches_delete_statement_at_top_level() {
    let mut parser = Parser::with_string("DELETE FROM foo;".to_owned()).unwrap();

    let statements = parser.parse(ParserContext::default()).unwrap();

    assert_eq!(statements.len(), 1);
    assert!(matches!(
        statements[0],
        SQLStatement::DML(DMLStatement::DeleteQuery(_))
    ));
}

#[test]
fn parse_dispatches_backslash_command_at_top_level() {
    let mut parser = Parser::with_string("\\l".to_owned()).unwrap();

    let statements = parser.parse(ParserContext::default()).unwrap();

    assert_eq!(statements.len(), 1);
    assert!(matches!(
        statements[0],
        SQLStatement::Other(OtherStatement::ShowDatabases(_))
    ));
}

#[test]
fn parse_dispatches_show_query_at_top_level() {
    let mut parser = Parser::with_string("SHOW DATABASES;".to_owned()).unwrap();

    let statements = parser.parse(ParserContext::default()).unwrap();

    assert_eq!(statements.len(), 1);
    assert!(matches!(
        statements[0],
        SQLStatement::Other(OtherStatement::ShowDatabases(_))
    ));
}

#[test]
fn parse_dispatches_use_query_at_top_level() {
    let mut parser = Parser::with_string("USE foo;".to_owned()).unwrap();

    let statements = parser.parse(ParserContext::default()).unwrap();

    assert_eq!(statements.len(), 1);
    assert!(matches!(
        statements[0],
        SQLStatement::Other(OtherStatement::UseDatabase(_))
    ));
}

#[test]
fn parse_dispatches_desc_query_at_top_level() {
    let mut parser = Parser::with_string("DESC foo;".to_owned()).unwrap();

    let statements = parser.parse(ParserContext::default()).unwrap();

    assert_eq!(statements.len(), 1);
    assert!(matches!(
        statements[0],
        SQLStatement::Other(OtherStatement::DescTable(_))
    ));
}

#[test]
fn parse_stops_at_an_unrecognized_top_level_token() {
    let mut parser = Parser::with_string("FROM foo;".to_owned()).unwrap();

    let statements = parser.parse(ParserContext::default()).unwrap();

    assert!(statements.is_empty());
}
