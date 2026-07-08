#![cfg(test)]

use crate::engine::ast::ddl::create_index::CreateIndexQuery;
use crate::engine::ast::ddl::drop_index::DropIndexQuery;
use crate::engine::ast::types::TableName;
use crate::engine::parser::context::ParserContext;
use crate::engine::parser::predule::Parser;

#[test]
pub fn create_index() {
    let text = r#"
        create index foo_id_idx on "foo_db".foo (id);
    "#
    .to_owned();

    let mut parser = Parser::with_string(text).unwrap();

    let expected = CreateIndexQuery::builder()
        .set_index_name("foo_id_idx".to_owned())
        .set_table(TableName::new(Some("foo_db".to_owned()), "foo".to_owned()))
        .add_column("id".to_owned())
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}

#[test]
pub fn create_unique_index_if_not_exists() {
    let text = r#"
        create unique index if not exists foo_id_idx on foo (id, name);
    "#
    .to_owned();

    let mut parser = Parser::with_string(text).unwrap();

    let expected = CreateIndexQuery::builder()
        .set_index_name("foo_id_idx".to_owned())
        .set_table(TableName::new(Some("rrdb".to_owned()), "foo".to_owned()))
        .add_column("id".to_owned())
        .add_column("name".to_owned())
        .set_unique(true)
        .set_if_not_exists(true)
        .build();

    assert_eq!(
        parser
            .parse(ParserContext::default().set_default_database("rrdb".to_owned()))
            .unwrap(),
        vec![expected],
    );
}

#[test]
pub fn create_index_without_semicolon() {
    // 입력 끝의 공백: 토크나이저가 EOF 직전의 ')'를 소실하는 기존 동작 우회
    let text = "create index foo_id_idx on foo (id) ".to_owned();

    let mut parser = Parser::with_string(text).unwrap();

    let expected = CreateIndexQuery::builder()
        .set_index_name("foo_id_idx".to_owned())
        .set_table(TableName::new(None, "foo".to_owned()))
        .add_column("id".to_owned())
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}

#[test]
pub fn create_index_errors() {
    // UNIQUE 뒤에 INDEX가 아닌 토큰
    assert!(
        Parser::with_string("create unique table foo;".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );

    // 인덱스명 누락
    assert!(
        Parser::with_string("create index on foo (id);".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );

    // ON 누락
    assert!(
        Parser::with_string("create index foo_idx foo (id);".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );

    // 여는 괄호 누락
    assert!(
        Parser::with_string("create index foo_idx on foo id;".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );

    // 컬럼 자리에 잘못된 토큰
    assert!(
        Parser::with_string("create index foo_idx on foo (1);".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );

    // 닫는 괄호 누락
    assert!(
        Parser::with_string("create index foo_idx on foo (id".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );

    // 세미콜론 자리에 잘못된 토큰
    assert!(
        Parser::with_string("create index foo_idx on foo (id) foo".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );

    // 빈 컬럼 리스트
    assert!(
        Parser::with_string("create index foo_idx on foo ();".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );
}

#[test]
pub fn drop_index() {
    let text = r#"
        drop index foo_id_idx;
    "#
    .to_owned();

    let mut parser = Parser::with_string(text).unwrap();

    let expected = DropIndexQuery::builder()
        .set_database_name(Some("rrdb".to_owned()))
        .set_index_name("foo_id_idx".to_owned())
        .build();

    assert_eq!(
        parser
            .parse(ParserContext::default().set_default_database("rrdb".to_owned()))
            .unwrap(),
        vec![expected],
    );
}

#[test]
pub fn drop_index_if_exists_with_on_clause() {
    let text = r#"
        drop index if exists foo_id_idx on "foo_db".foo;
    "#
    .to_owned();

    let mut parser = Parser::with_string(text).unwrap();

    let expected = DropIndexQuery::builder()
        .set_index_name("foo_id_idx".to_owned())
        .set_table(TableName::new(Some("foo_db".to_owned()), "foo".to_owned()))
        .set_if_exists(true)
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}

#[test]
pub fn drop_index_without_semicolon() {
    let text = "drop index foo_id_idx".to_owned();

    let mut parser = Parser::with_string(text).unwrap();

    let expected = DropIndexQuery::builder()
        .set_index_name("foo_id_idx".to_owned())
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}

#[test]
pub fn drop_index_errors() {
    // 인덱스명 누락
    assert!(
        Parser::with_string("drop index;".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );

    // 인덱스명 뒤에 잘못된 토큰
    assert!(
        Parser::with_string("drop index foo_idx foo;".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );

    // ON 뒤 테이블명 다음에 잘못된 토큰
    assert!(
        Parser::with_string("drop index foo_idx on foo foo".to_owned())
            .unwrap()
            .parse(ParserContext::default())
            .is_err()
    );
}
