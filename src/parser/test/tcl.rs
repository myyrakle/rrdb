#![cfg(test)]

use crate::{
    ast::{
        tcl::{BeginTransactionQuery, CommitQuery, RollbackQuery},
        SQLStatement,
    },
    parser::predule::{Parser, ParserContext},
};

#[test]
pub fn begin_transaction() {
    struct TestCase {
        name: String,
        input: String,
        expected: SQLStatement,
        want_err: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "정상적인 트랜잭션 시작".to_owned(),
            input: "BEGIN TRANSACTION;".to_owned(),
            expected: BeginTransactionQuery {}.into(),
            want_err: false,
        },
        TestCase {
            name: "begin만 있는 경우".to_owned(),
            input: "BEGIN;".to_owned(),
            expected: Default::default(),
            want_err: true,
        },
        TestCase {
            name: "begin 이후에 기대하지 않은 입력이 있는 경우".to_owned(),
            input: "BEGIN TRANSITION;".to_owned(),
            expected: Default::default(),
            want_err: true,
        },
    ];

    for tc in test_cases {
        let mut parser = Parser::with_string(tc.input).unwrap();

        let result = parser.parse(ParserContext::default());

        if tc.want_err {
            assert!(
                result.is_err(),
                "{} - expected error, got {:?}",
                tc.name,
                result
            );
            continue;
        }

        assert_eq!(result.unwrap(), vec![tc.expected], "{}", tc.name);
    }
}

#[test]
pub fn commit() {
    struct TestCase {
        name: String,
        input: String,
        expected: SQLStatement,
        want_err: bool,
    }

    let test_cases = vec![TestCase {
        name: "정상적인 Commit 명령".to_owned(),
        input: "COMMIT;".to_owned(),
        expected: CommitQuery {}.into(),
        want_err: false,
    }];

    for tc in test_cases {
        let mut parser = Parser::with_string(tc.input).unwrap();

        let result = parser.parse(ParserContext::default());

        if tc.want_err {
            assert!(
                result.is_err(),
                "{} - expected error, got {:?}",
                tc.name,
                result
            );
            continue;
        }

        assert_eq!(result.unwrap(), vec![tc.expected], "{}", tc.name);
    }
}

#[test]
pub fn rollback() {
    struct TestCase {
        name: String,
        input: String,
        expected: SQLStatement,
        want_err: bool,
    }

    let test_cases = vec![TestCase {
        name: "정상적인 ROLLBACK 명령".to_owned(),
        input: "ROLLBACK;".to_owned(),
        expected: RollbackQuery {}.into(),
        want_err: false,
    }];

    for tc in test_cases {
        let mut parser = Parser::with_string(tc.input).unwrap();

        let result = parser.parse(ParserContext::default());

        if tc.want_err {
            assert!(
                result.is_err(),
                "{} - expected error, got {:?}",
                tc.name,
                result
            );
            continue;
        }

        assert_eq!(result.unwrap(), vec![tc.expected], "{}", tc.name);
    }
}
