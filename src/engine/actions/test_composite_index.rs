//! Task 2 RED 테스트: 복합 인덱스 키 지원 (#220)
//!
//! - IndexMeta.columns (복수 컬럼) 지원
//! - row_index_keys: 복수 컬럼 키 인코딩 (순서 보존, 단사)
//! - NULL 포함 컬럼 → 인덱스 제외 (PostgreSQL과 동일)

use crate::engine::actions::index::row_index_keys;
use crate::engine::ast::types::TableName;
use crate::engine::index::{IndexMeta, field_to_key};
use crate::engine::schema::row::{TableDataField, TableDataFieldType, TableDataRow};

fn table() -> TableName {
    TableName::new(Some("rrdb".to_owned()), "memberships".to_owned())
}

fn field(name: &str, data: TableDataFieldType) -> TableDataField {
    TableDataField {
        table_name: table(),
        column_name: name.to_owned(),
        data,
    }
}

#[test]
fn index_meta_supports_multiple_columns() {
    let meta = IndexMeta::new_composite(
        "rrdb.memberships_pkey".to_owned(),
        table(),
        vec!["user_id".to_owned(), "group_id".to_owned()],
        true,
    );

    assert_eq!(meta.columns(), vec!["user_id", "group_id"]);
    // 하위 호환: 단일 컬럼 인덱스는 column_name()로 첫 컬럼을 반환
    let single = IndexMeta::new("rrdb.users_pkey".to_owned(), table(), "id".to_owned(), true);
    assert_eq!(single.columns(), vec!["id"]);
    assert_eq!(single.column_name(), "id");
}

#[test]
fn composite_key_is_concatenation_with_unambiguous_boundaries() {
    let row = TableDataRow {
        fields: vec![
            field("user_id", TableDataFieldType::Integer(1)),
            field("group_id", TableDataFieldType::Integer(2)),
        ],
    };

    let keys = row_index_keys(&row, &["user_id".to_owned(), "group_id".to_owned()]).unwrap();

    // 두 키를 합친 값이 단사여야 함: 다른 컬럼 조합이 같은 키를 만들 수 없음
    let combined = keys.join("");
    assert!(combined.contains(&field_to_key(&TableDataFieldType::Integer(1))));
    assert!(combined.contains(&field_to_key(&TableDataFieldType::Integer(2))));
}

#[test]
fn composite_key_encoding_is_injective_across_value_shapes() {
    use crate::engine::actions::index::join_composite_key;

    // "S:ab" + "S:c" vs "S:a" + "S:bc" 가 같은 키가 되는 충돌을 막아야 함
    let row_a = TableDataRow {
        fields: vec![
            field("x", TableDataFieldType::String("ab".to_owned())),
            field("y", TableDataFieldType::String("c".to_owned())),
        ],
    };
    let row_b = TableDataRow {
        fields: vec![
            field("x", TableDataFieldType::String("a".to_owned())),
            field("y", TableDataFieldType::String("bc".to_owned())),
        ],
    };

    let keys_a = row_index_keys(&row_a, &["x".to_owned(), "y".to_owned()]).unwrap();
    let keys_b = row_index_keys(&row_b, &["x".to_owned(), "y".to_owned()]).unwrap();

    assert_ne!(
        keys_a, keys_b,
        "different value splits must not collide: {:?} vs {:?}",
        keys_a, keys_b
    );

    // 조합된 B-tree 키도 충돌 없어야 함 (길이 프리픽스 인코딩)
    assert_ne!(
        join_composite_key(&keys_a),
        join_composite_key(&keys_b),
        "joined composite keys must not collide"
    );
}

#[test]
fn composite_key_preserves_column_order() {
    let row = TableDataRow {
        fields: vec![
            field("a", TableDataFieldType::Integer(1)),
            field("b", TableDataFieldType::Integer(2)),
        ],
    };

    let ab = row_index_keys(&row, &["a".to_owned(), "b".to_owned()]).unwrap();
    let ba = row_index_keys(&row, &["b".to_owned(), "a".to_owned()]).unwrap();

    assert_ne!(ab, ba, "column order must affect the key");
    // 컴포넌트는 field_to_key 원본 키
    assert_eq!(ab[0], field_to_key(&TableDataFieldType::Integer(1)));
    assert_eq!(ab[1], field_to_key(&TableDataFieldType::Integer(2)));
    assert_eq!(ba[0], field_to_key(&TableDataFieldType::Integer(2)));
}

#[test]
fn row_with_null_in_indexed_column_is_not_indexed() {
    let row = TableDataRow {
        fields: vec![
            field("user_id", TableDataFieldType::Integer(1)),
            field("group_id", TableDataFieldType::Null),
        ],
    };

    assert!(
        row_index_keys(&row, &["user_id".to_owned(), "group_id".to_owned()]).is_none(),
        "NULL이 포함된 복합 키는 색인하지 않음 (PostgreSQL과 동일)"
    );
}

#[test]
fn row_missing_indexed_column_is_not_indexed() {
    let row = TableDataRow {
        fields: vec![field("user_id", TableDataFieldType::Integer(1))],
    };

    assert!(row_index_keys(&row, &["user_id".to_owned(), "missing".to_owned()]).is_none());
}

#[test]
fn test_single_column_row_index_keys_matches_row_index_key() {
    use crate::engine::actions::index::{join_composite_key, row_index_key, row_index_keys};

    let row = TableDataRow {
        fields: vec![field("id", TableDataFieldType::Integer(42))],
    };

    let via_multi = row_index_keys(&row, &["id".to_owned()]).unwrap();
    let via_single = row_index_key(&row, "id").unwrap();

    // 단일 컬럼 복합 키를 join하면 기존 row_index_key와 정확히 일치해야 함:
    // 단일 인덱스 경로가 동일 키 공간(옵티마이저 eq_key 포함)을 유지하는 계약 (#220)
    assert_eq!(join_composite_key(&via_multi), via_single);
}
