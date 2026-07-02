# RRDB Engine AGENTS.md

## 개요

Engine 모듈은 SQL 텍스트를 받아 실행 가능한 액션으로 변환하고 결과를 반환하는 RRDB의 코어입니다.

## 아키텍처: 쿼리 처리 파이프라인

```text
SQL Text (String)
    │
    ▼
┌──────────┐
│  lexer   │  ← SQL 문자열 → 토큰 벡터 (Tokenize)
└────┬─────┘
     │ tokens: Vec<Token>
     ▼
┌──────────┐
│  parser  │  ← 토큰 → AST (SQLStatement)
└────┬─────┘
     │ AST: DDLStatement | DMLStatement | DCLStatement | TCLStatement | OtherStatement | None
     ▼
┌───────────┐
│ optimizer │  ← AST 정규화/최적화 (선택적)
└────┬──────┘
     │
     ▼
┌──────────┐
│  actions │  ← AST → 실제 DB 조작 (DDL/DML/DCL/TCL/Other)
│ DBEngine │     각 match arm에서 실행
└────┬─────┘
     │ WAL 기록 (wal_enabled = true 시)
     ▼
┌──────────────┐
│  ExecuteResult│  ← ExecuteField 벡터 (Bool/Int/Float/String/Null)
└──────────────┘
```

## 모듈 책임

### `ast/` — 추상 구문 트리
- `SQLStatement`: DDL / DML / DCL / TCL / Other / None 6가지 변형
- `DDLStatement`: CREATE DATABASE, CREATE TABLE, ALTER DATABASE, ALTER TABLE, DROP DATABASE, DROP TABLE, CREATE INDEX
- `DCLStatement`: (현재 빈 enum, 향후 GRANT/REVOKE 등)
- `TCLStatement`: BEGIN TRANSACTION, COMMIT, ROLLBACK
- `DMLStatement`: INSERT, SELECT, UPDATE, DELETE
- `OtherStatement`: SHOW DATABASES, USE DATABASE, SHOW TABLES, DESC TABLE
- `TableName { database_name: Option<String>, table_name: String }`

### `lexer/` — 어휘 분석
- SQL 문자열 → `Vec<Token>` 변환
- 키워드, 식별자, 리터럴, 연산자, 구분자 인식
- 에러: `LexingError` (`ErrorKind::LexingError`)

### `parser/` — 구문 분석
- 토큰 스트림 → `SQLStatement` AST
- LL(k) 스타일 재귀 하강 파서
- 에러: `ParsingError` (`ErrorKind::ParsingError`)

### `optimizer/` — AST 최적화
- 조건절 단순화, 불필요한 연산 제거
- 쿼리 실행 계획 정규화
- 현재는 기본 구조만 갖춤

### `actions/` — DB 액션 실행
- `DBEngine`이 직접 구현한 DDL/DML 메서드들
- `create_database`, `create_table`, `insert`, `select` 등
- 각 action은 내부적으로 `FileSystem` 트레이트로 I/O

### `expression/` — 표현식 평가
- 조건식, 연산식 평가 로직
- WHERE 절 필터링

### `types/` — 타입 시스템
- `ExecuteResult`, `ExecuteField` (Bool/Int/Float/String/Null)
- SQL 실행 결과 표현

### `schema/` — 스키마 관리
- `TableSchema` 및 storage format
- 테이블 config (`table.config`) 인코딩/디코딩
- 인코더: `StorageEncoder` (schema_encoder)

### `server/` — TCP 서버 레이어
- `Server`: TCP listener + connection accept
- `channel`: Server Background Loop ↔ per-connection 통신용 `oneshot` 채널
- `shared_state`: `SharedState` (클라이언트 정보 + 채널 sender)

### `wal/` — Write-Ahead Logging
- `WALManager` + `BitcodeEncoder`
- WAL 세그먼트: 크기 제한 (`wal_segment_size`), 확장자 (`wal_extension`)
- 복구 시 WAL replay

### `encoder/` — 인코딩 유틸리티
- `schema_encoder`: StorageEncoder — 스키마 데이터 bitcode 직렬화

### `initialize/` — 초기화
- `DBEngine::initialize_with_base_path()` — 데이터 저장소 초기 설정

## AST 패턴 매칭 (DBEngine::process_query)

```rust
pub async fn process_query(
    &self,
    statement: SQLStatement,
    _wal_manager: Arc<WALManager<BitcodeEncoder>>,
    _connection_id: String,
) -> errors::Result<ExecuteResult> {
    let result = match statement {
        // ── DDL ──────────────────────────────────────────
        SQLStatement::DDL(DDLStatement::CreateDatabaseQuery(query))   => self.create_database(query).await,
        SQLStatement::DDL(DDLStatement::AlterDatabase(query))         => self.alter_database(query).await,
        SQLStatement::DDL(DDLStatement::DropDatabaseQuery(query))     => self.drop_database(query).await,
        SQLStatement::DDL(DDLStatement::CreateTableQuery(query))      => self.create_table(query).await,
        SQLStatement::DDL(DDLStatement::AlterTableQuery(query))       => self.alter_table(query).await,
        SQLStatement::DDL(DDLStatement::DropTableQuery(query))        => self.drop_table(query).await,

        // ── DML ──────────────────────────────────────────
        SQLStatement::DML(DMLStatement::InsertQuery(query))            => self.insert(query).await,
        SQLStatement::DML(DMLStatement::SelectQuery(query))            => self.select(query).await,
        SQLStatement::DML(DMLStatement::UpdateQuery(query))            => self.update(query).await,
        SQLStatement::DML(DMLStatement::DeleteQuery(query))            => self.delete(query).await,

        // ── Other ────────────────────────────────────────
        SQLStatement::Other(OtherStatement::ShowDatabases(query))      => self.show_databases(query).await,
        SQLStatement::Other(OtherStatement::UseDatabase(query))        => self.use_databases(query).await,
        SQLStatement::Other(OtherStatement::ShowTables(query))         => self.show_tables(query).await,
        SQLStatement::Other(OtherStatement::DescTable(query))          => self.desc_table(query).await,

        _ => unimplemented!("no execute implementation"),
    };

    match result {
        Ok(result) => Ok(result),
        Err(error) => Err(ExecuteError::wrap(error.to_string())),
    }
}
```

## WAL 통합 패턴

```rust
// 1. WALManager는 Arc로 공유
wal_manager: Arc<WALManager<BitcodeEncoder>>

// 2. mutation 작업(INSERT/UPDATE/DELETE/CREATE/DROP) 시 WAL 기록
//    engine actions 내부에서 wal_manager.append() 호출

// 3. WAL 세그먼트 속성
//    - segment_size: config.wal_segment_size (기본 16MB)
//    - extension: config.wal_extension (기본 "log")
//    - encoder: BitcodeEncoder (bitcode crate)

// 4. WAL 복구는 Server startup 시 수행
```

## 주의사항

- **AST 패턴 매칭은 exhaustive**: `SQLStatement`에 새 변형 추가 시 반드시 `process_query`의 match에 추가
- **에러 변환**: actions에서 발생한 에러는 `ExecuteError::wrap()`으로 감싸서 반환
- **스키마 인코딩**: `StorageEncoder` (`bitcode`)로 테이블 config 저장/로드
- **파일시스템 접근**: 직접 `tokio::fs` 호출 대신 `self.file_system` 트레이트 메서드 사용하여 테스트 가능하게 유지
- **채널 통신**: Server는 `SharedState`의 `oneshot` 채널로 per-connection 요청을 dispatch
