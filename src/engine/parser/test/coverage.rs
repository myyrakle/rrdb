#![cfg(test)]

use std::collections::VecDeque;

use crate::engine::ast::dml::parts::join::JoinType;
use crate::engine::ast::other::show_tables::ShowTablesQuery;
use crate::engine::ast::types::DataType;
use crate::engine::lexer::tokens::Token;
use crate::engine::parser::predule::{Parser, ParserContext};

#[test]
fn parser_parse_handles_remaining_top_level_branches_in_one_stream() {
    let mut ddl_parser = Parser::with_string(
        "; CREATE DATABASE db1; ALTER DATABASE db1; DROP DATABASE db1;".to_owned(),
    )
    .unwrap();
    let ddl_statements = ddl_parser.parse(ParserContext::default()).unwrap();
    assert_eq!(ddl_statements.len(), 3);

    let mut tcl_parser =
        Parser::with_string("BEGIN TRANSACTION; COMMIT; ROLLBACK;".to_owned()).unwrap();
    let tcl_statements = tcl_parser.parse(ParserContext::default()).unwrap();
    assert_eq!(tcl_statements.len(), 3);
}

#[test]
fn parser_parse_breaks_on_unrecognized_top_level_token_from_raw_tokens() {
    let mut parser = Parser::with_tokens(VecDeque::from(vec![Token::Null]));

    let statements = parser.parse(ParserContext::default()).unwrap();

    assert!(statements.is_empty());
}

#[test]
fn parser_new_keeps_tokens_available() {
    let parser = Parser::new(vec![Token::Select, Token::EOF]);

    assert_eq!(parser.current_token, Token::EOF);
    assert!(parser.has_next_token());
}

#[test]
fn parse_show_query_uses_none_when_default_database_is_missing() {
    let mut parser = Parser::new(vec![Token::Tables]);

    let statement = parser.parse_show_query(ParserContext::default()).unwrap();

    assert_eq!(
        statement,
        ShowTablesQuery {
            database: "None".into(),
        }
        .into()
    );
}

#[test]
fn common_helpers_cover_bool_float_and_right_parenthesis_column_exit() {
    let mut bool_parser = Parser::new(vec![Token::Identifier("BOOL".into())]);
    assert_eq!(bool_parser.parse_data_type().unwrap(), DataType::Boolean);

    let mut float_parser = Parser::new(vec![Token::Identifier("FLOAT".into())]);
    assert_eq!(float_parser.parse_data_type().unwrap(), DataType::Float);

    let mut column_parser = Parser::new(vec![
        Token::Identifier("flag".into()),
        Token::Identifier("BOOL".into()),
        Token::RightParentheses,
    ]);
    let column = column_parser.parse_table_column().unwrap();
    assert_eq!(column.data_type, DataType::Boolean);
    assert_eq!(column_parser.get_next_token(), Token::RightParentheses);
}

#[test]
fn database_and_table_parsers_cover_optional_variants() {
    let mut alter_db = Parser::new(vec![Token::Identifier("db1".into()), Token::SemiColon]);
    assert!(alter_db.handle_alter_database_query().is_ok());

    let mut create_table = Parser::new(vec![
        Token::If,
        Token::Not,
        Token::Exists,
        Token::Identifier("foo".into()),
        Token::LeftParentheses,
        Token::Identifier("id".into()),
        Token::Identifier("INT".into()),
        Token::RightParentheses,
        Token::SemiColon,
    ]);
    assert!(
        create_table
            .handle_create_table_query(ParserContext::default())
            .is_ok()
    );

    let mut alter_table = Parser::new(vec![Token::Identifier("foo".into()), Token::SemiColon]);
    assert!(
        alter_table
            .handle_alter_table_query(ParserContext::default())
            .is_ok()
    );

    let mut add_without_column_keyword = Parser::new(vec![
        Token::Identifier("foo".into()),
        Token::Add,
        Token::Identifier("bar".into()),
        Token::Identifier("INT".into()),
    ]);
    assert!(
        add_without_column_keyword
            .handle_alter_table_query(ParserContext::default())
            .is_ok()
    );

    let mut alter_type_keyword = Parser::new(vec![
        Token::Identifier("foo".into()),
        Token::Alter,
        Token::Identifier("bar".into()),
        Token::Type,
        Token::Identifier("BOOL".into()),
    ]);
    assert!(
        alter_type_keyword
            .handle_alter_table_query(ParserContext::default())
            .is_ok()
    );

    let mut drop_table = Parser::new(vec![
        Token::If,
        Token::Exists,
        Token::Identifier("foo".into()),
        Token::SemiColon,
    ]);
    assert!(
        drop_table
            .handle_drop_table_query(ParserContext::default())
            .is_ok()
    );
}

#[test]
fn dml_parsers_cover_alias_where_and_insert_select_paths() {
    let mut delete_parser =
        Parser::with_string("DELETE FROM foo f WHERE 1 = 1;".to_owned()).unwrap();
    assert!(delete_parser.parse(ParserContext::default()).is_ok());

    let mut update_parser =
        Parser::with_string("UPDATE foo f SET id = 1 WHERE 1 = 1;".to_owned()).unwrap();
    assert!(update_parser.parse(ParserContext::default()).is_ok());

    let mut insert_select =
        Parser::with_string("INSERT INTO foo (id) SELECT id FROM bar;".to_owned()).unwrap();
    assert!(insert_select.parse(ParserContext::default()).is_ok());

    let mut values_parser = Parser::new(vec![
        Token::Values,
        Token::LeftParentheses,
        Token::Integer(1),
        Token::RightParentheses,
    ]);
    let values = values_parser
        .parse_insert_values(ParserContext::default())
        .unwrap();
    assert!(values[0].list[0].is_some());
}

#[test]
fn select_parser_covers_subquery_alias_join_having_and_limit_offset_orders() {
    let mut subquery = Parser::with_string("SELECT * FROM (SELECT 1) s;".to_owned()).unwrap();
    assert!(subquery.parse(ParserContext::default()).is_ok());

    let mut join_query = Parser::with_string(
        "SELECT a FROM foo f INNER JOIN bar b ON 1 = 1 WHERE 1 = 1;".to_owned(),
    )
    .unwrap();
    assert!(join_query.parse(ParserContext::default()).is_ok());

    let mut having_limit_offset = Parser::with_string(
        "SELECT a FROM foo GROUP BY a HAVING a = 1 ORDER BY a DESC NULLS LAST LIMIT 10 OFFSET 2;"
            .to_owned(),
    )
    .unwrap();
    assert!(having_limit_offset.parse(ParserContext::default()).is_ok());

    let mut offset_limit =
        Parser::with_string("SELECT a FROM foo OFFSET 2 LIMIT 10;".to_owned()).unwrap();
    assert!(offset_limit.parse(ParserContext::default()).is_ok());
}

#[test]
fn select_helpers_cover_right_parenthesis_exit_paths() {
    let mut group_by_parser = Parser::new(vec![
        Token::Select,
        Token::Identifier("a".into()),
        Token::From,
        Token::Identifier("foo".into()),
        Token::Group,
        Token::By,
        Token::Identifier("a".into()),
        Token::RightParentheses,
    ]);
    assert!(
        group_by_parser
            .handle_select_query(ParserContext::default())
            .is_ok()
    );
    assert_eq!(group_by_parser.get_next_token(), Token::RightParentheses);

    let mut order_by_parser = Parser::new(vec![
        Token::Select,
        Token::Identifier("a".into()),
        Token::From,
        Token::Identifier("foo".into()),
        Token::Order,
        Token::By,
        Token::Identifier("a".into()),
        Token::RightParentheses,
    ]);
    assert!(
        order_by_parser
            .handle_select_query(ParserContext::default())
            .is_ok()
    );
    assert_eq!(order_by_parser.get_next_token(), Token::RightParentheses);
}

#[test]
fn expression_parser_covers_literal_binary_between_subquery_and_function_paths() {
    let cases = [
        "1.5 + 2.0",
        "1.5 BETWEEN 1 AND 2",
        "'a' = 'b'",
        "'a' BETWEEN 'a' AND 'z'",
        "true = false",
        "true BETWEEN false AND true",
        "NULL = NULL",
        "NULL BETWEEN NULL AND NULL",
        "NOT 1 = 1",
        "(SELECT 1) = 1",
        "1 + 2 * 3",
        "-1",
        "+1",
    ];

    for sql in cases {
        let mut parser = Parser::with_string(sql.to_owned()).unwrap();
        assert!(
            parser.parse_expression(ParserContext::default()).is_ok(),
            "{sql}"
        );
    }

    let mut right_paren = Parser::new(vec![Token::RightParentheses]);
    assert!(
        right_paren
            .parse_expression(ParserContext::default())
            .is_err()
    );
}

#[test]
fn select_and_join_helpers_are_callable_directly() {
    let mut join_parser = Parser::new(vec![
        Token::Identifier("bar".into()),
        Token::Identifier("b".into()),
        Token::On,
        Token::Integer(1),
        Token::Operator(crate::engine::lexer::operator_token::OperatorToken::Eq),
        Token::Integer(1),
    ]);
    let join = join_parser
        .parse_join(JoinType::InnerJoin, ParserContext::default())
        .unwrap();
    assert!(join.right_alias.is_some());
    assert!(join.on.is_some());

    let mut where_parser = Parser::with_string("WHERE 1 = 1".to_owned()).unwrap();
    assert!(where_parser.parse_where(ParserContext::default()).is_ok());

    let mut having_parser = Parser::with_string("HAVING 1 = 1".to_owned()).unwrap();
    assert!(having_parser.parse_having(ParserContext::default()).is_ok());

    let mut limit_parser = Parser::with_string("LIMIT 3".to_owned()).unwrap();
    assert_eq!(
        limit_parser.parse_limit(ParserContext::default()).unwrap(),
        3
    );

    let mut offset_parser = Parser::with_string("OFFSET 2".to_owned()).unwrap();
    assert_eq!(
        offset_parser
            .parse_offset(ParserContext::default())
            .unwrap(),
        2
    );
}

#[test]
fn show_tokens_can_be_called_from_tests() {
    let parser = Parser::new(vec![Token::Select, Token::EOF]);
    parser.show_tokens();
}

#[test]
fn insert_parser_covers_multiple_value_tuples_and_default() {
    let mut default_values = Parser::new(vec![
        Token::Insert,
        Token::Into,
        Token::Identifier("foo".into()),
        Token::LeftParentheses,
        Token::Identifier("id".into()),
        Token::RightParentheses,
        Token::Values,
        Token::LeftParentheses,
        Token::Default,
        Token::RightParentheses,
    ]);
    assert!(
        default_values
            .handle_insert_query(ParserContext::default())
            .is_ok()
    );

    let mut expression_values = Parser::new(vec![
        Token::Insert,
        Token::Into,
        Token::Identifier("foo".into()),
        Token::LeftParentheses,
        Token::Identifier("id".into()),
        Token::RightParentheses,
        Token::Values,
        Token::LeftParentheses,
        Token::Integer(1),
        Token::Operator(crate::engine::lexer::operator_token::OperatorToken::Plus),
        Token::Integer(2),
        Token::RightParentheses,
    ]);
    assert!(
        expression_values
            .handle_insert_query(ParserContext::default())
            .is_ok()
    );

    let mut comma_values = Parser::new(vec![
        Token::Values,
        Token::LeftParentheses,
        Token::Integer(1),
        Token::Comma,
        Token::Integer(2),
        Token::RightParentheses,
    ]);
    assert!(
        comma_values
            .parse_insert_values(ParserContext::default())
            .is_ok()
    );
}

#[test]
fn common_helpers_cover_parse_table_name_error_paths() {
    let mut no_token = Parser::new(vec![]);
    assert!(no_token.parse_table_name(ParserContext::default()).is_err());

    let mut not_identifier = Parser::new(vec![Token::SemiColon]);
    assert!(
        not_identifier
            .parse_table_name(ParserContext::default())
            .is_err()
    );

    let mut run_out_after_period =
        Parser::new(vec![Token::Identifier("foo".into()), Token::Period]);
    assert!(
        run_out_after_period
            .parse_table_name(ParserContext::default())
            .is_err()
    );

    let mut join_outer_no_next = Parser::new(vec![Token::Left]);
    assert_eq!(join_outer_no_next.get_next_join_type(), None);
}

#[test]
fn expression_parser_covers_qualified_function_call_path() {
    let mut parser = Parser::with_string("db1.foo(1);".to_owned()).unwrap();
    let expression = parser.parse_expression(ParserContext::default()).unwrap();

    match expression {
        crate::engine::ast::types::SQLExpression::FunctionCall(call) => {
            assert_eq!(
                call.function,
                crate::engine::ast::types::UserDefinedFunction {
                    database_name: Some("db1".into()),
                    function_name: "foo".into(),
                }
                .into()
            );
        }
        other => panic!("expected a call expression, got {other:?}"),
    }
}

#[test]
fn expression_parser_propagates_error_from_an_unterminated_function_call() {
    let mut parser = Parser::new(vec![
        Token::Identifier("foo".into()),
        Token::LeftParentheses,
    ]);

    assert!(parser.parse_expression(ParserContext::default()).is_err());
}

#[test]
fn insert_values_parser_skips_non_expression_tokens_inside_a_tuple() {
    let mut parser = Parser::new(vec![
        Token::Values,
        Token::LeftParentheses,
        Token::From,
        Token::RightParentheses,
    ]);

    let values = parser
        .parse_insert_values(ParserContext::default())
        .unwrap();
    assert!(values[0].list.is_empty());
}

#[test]
fn select_parser_covers_empty_select_item_list_before_from() {
    let mut parser = Parser::with_string("SELECT FROM foo;".to_owned()).unwrap();
    let statements = parser.parse(ParserContext::default()).unwrap();

    assert_eq!(statements.len(), 1);
}

#[test]
fn select_parser_covers_select_item_error_paths_and_more_variants() {
    let mut select_as_identifier =
        Parser::with_string("SELECT 1 AS foo FROM bar".to_owned()).unwrap();
    assert!(select_as_identifier.parse(ParserContext::default()).is_ok());

    let mut where_expression =
        Parser::with_string("SELECT * FROM bar WHERE 1 = 1 + 2".to_owned()).unwrap();
    assert!(where_expression.parse(ParserContext::default()).is_ok());

    let mut join_without_on =
        Parser::with_string("SELECT * FROM foo INNER JOIN bar".to_owned()).unwrap();
    assert!(join_without_on.parse(ParserContext::default()).is_ok());

    let mut semi_after_select =
        Parser::new(vec![Token::Select, Token::Integer(1), Token::SemiColon]);
    assert!(
        semi_after_select
            .handle_select_query(ParserContext::default())
            .is_ok()
    );
}
