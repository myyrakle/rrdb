# RRDB pgwire AGENTS.md

## 개요

pgwire 모듈은 **PostgreSQL Wire Protocol v3**를 구현합니다. 표준 PostgreSQL 클라이언트(`psql`, libpq 등)가 RRDB에 연결할 수 있도록 합니다.

## 아키텍처

```
TCP Client (psql/libpq)
    │
    ▼
┌──────────────────┐
│  connection/      │  ← TcpStream accept, per-connection task
│  (tokio::spawn)   │     ConnectionCodec로 메시지 디코딩
└──────┬───────────┘
       │ ClientMessage
       ▼
┌──────────────────┐
│  protocol/        │  ← 메시지 타입, 코덱, 데이터 타입
│  (codec + types)  │     BackendMessage/ClientMessage 정의
└──────┬───────────┘
       │
       ▼
┌──────────────────┐
│  engine/          │  ← Engine trait 구현
│  (RRDBEngine)     │     prepare() → FieldDescription
│                   │     create_portal() → Portal → DataRowBatch
└──────┬───────────┘
       │ ChannelRequest (oneshot)
       ▼
┌──────────────────┐
│  engine/server/   │  ← Background Loop에서 쿼리 처리
│  (shared_state)   │     Engine.process_query() 호출
└──────────────────┘
```

## 모듈 책임

### `connection/`
- TCP 연결 수락 및 per-connection 태스크 생성
- `tokio::spawn`으로 각 클라이언트를 독립적으로 처리

### `engine/` — Engine Trait
RRDB 자체 Engine 구현:

```rust
#[async_trait]
pub trait Engine: Send + Sync + 'static {
    type PortalType: Portal;

    // statement prepare → FieldDescription 목록 반환
    async fn prepare(
        &mut self,
        statement: &SQLStatement,
    ) -> Result<Vec<FieldDescription>, ErrorResponse>;

    // prepare된 statement로 Portal 생성
    async fn create_portal(
        &mut self,
        stmt: &SQLStatement,
    ) -> Result<Self::PortalType, ErrorResponse>;
}
```

#### 구현체: `RRDBEngine`
- `SharedState` 보유 → `oneshot` 채널로 Engine Server에 요청 전달
- `prepare()`: ChannelRequest 발송 → 응답에서 FieldDescription 추출
- `create_portal()`: 준비된 `RRDBPortal` 반환 (또는 "not prepared yet" 에러)

#### `RRDBPortal`
```rust
pub struct RRDBPortal {
    pub shared_state: SharedState,
    pub execute_result: ExecuteResult,
}
```
- `fetch()`: `ExecuteResult.rows` → `DataRowBatch`에 field-by-field 쓰기
- 지원 필드 타입: `Bool`, `Integer`(int8), `Float`(float8), `String`, `Null`

### `protocol/` — PostgreSQL Wire Protocol 상세

#### 메시지 모듈 (`protocol/message/`)

**Client Messages** (`client/`):
| 메시지 | 타입 | 설명 |
|---|---|---|
| `Startup` | Startup | 연결 시작 (프로토콜 버전, 파라미터) |
| `Parse` | Parse | SQL 구문 분석 요청 |
| `Bind` | Bind | 파라미터 바인딩 |
| `Describe` | Describe | prepared statement/portal 설명 |
| `Execute` | Execute | 실행 요청 |
| `Close` | Close | statement/portal 종료 |

**Backend Messages** (`backend/`):
| 메시지 | 타입 | 설명 |
|---|---|---|
| `ErrorResponse` | ErrorResponse | 에러 응답 (severity + code + message) |
| `ReadyForQuery` | ReadyForQuery | 트랜잭션 상태 + 준비 완료 |
| `CommandComplete` | CommandComplete | 명령 완료 태그 |
| `DataRow` | (via DataRowBatch) | 결과 행 데이터 |
| `ParseComplete` | ParseComplete | Parse 성공 |
| `BindComplete` | BindComplete | Bind 성공 |
| `CloseComplete` | CloseComplete | Close 성공 |
| `ParameterStatus` | ParameterStatus | 서버 파라미터 알림 |
| `ParameterDescription` | ParameterDescription | 파라미터 타입 설명 |
| `RowDescription` | RowDescription | 결과 컬럼 설명 |
| `FieldDescription` | FieldDescription | 개별 필드 정보 |
| `EmptyQueryResponse` | EmptyQueryResponse | 빈 쿼리 응답 |
| `NoData` | NoData | 데이터 없음 |
| `ReadyForQuery` | ReadyForQuery | 트랜잭션 상태 보고 |

#### 코덱 (`protocol/connection_codec.rs`)
- `ConnectionCodec`: `tokio_util::codec::Decoder + Encoder` 구현
- Decoder: `BytesMut` → `ClientMessage`
- Encoder: `BackendMessage` → `BytesMut`
- Startup 메시지는 별도 헤더 포맷 (4바이트 길이 + 버전/파라미터 맵)
- 일반 메시지: 1바이트 타입 + 4바이트 길이 + payload

#### 데이터 타입 (`protocol/data_types.rs`)
- PostgreSQL 타입 ↔ Rust 타입 매핑

#### 포맷 코드 (`protocol/format_code.rs`)
- 텍스트(0) / 바이너리(1)

#### SQL State (`protocol/sql_state.rs`)
- PostgreSQL SQLSTATE 코드 정의 (5자리 코드)

#### Severity (`protocol/severity.rs`)
- `ERROR`, `FATAL`, `PANIC` 등

#### Extension (`protocol/extension/`)
- `DataRowWriter`: 행 데이터 쓰기
- `DataRowBatch`: 배치 데이터 전송

## 연결 수명 주기

```
1. TCP Connection Establishment
   │
2. Startup Message 수신 (프로토콜 버전, user, database)
   │
3. Authentication (현재는 간소화)
   │
4. ParameterStatus + ReadyForQuery (Backend → Client)
   │
   ═══════ Simple Query Mode ═══════
   │
5. Query 메시지 처리 루프:
   a. Parse (SQL 파싱 요청)
   b. Bind (파라미터 바인딩)
   c. Describe (결과 설명)
   d. Execute (실행)
   e. ReadyForQuery (준비 완료)
   │
   ═══════ Connection Close ═══════
   │
6. Terminate 메시지 → 연결 종료
```

## 에러 처리 (pgwire 컨텍스트)

```rust
// ProtocolError: thiserror enum
#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    Io(#[from] std::io::Error),
    Utf8(#[from] std::string::FromUtf8Error),
    ParserError,
    InvalidMessageType(u8),
    InvalidFormatCode(i16),
}

// ErrorResponse: BackendMessage → 클라이언트로 전송
ErrorResponse::fatal(SqlState::CONNECTION_EXCEPTION, "message")
ErrorResponse::error(SqlState::SYNTAX_ERROR, "message")
```

- `ProtocolError`는 wire 레벨의 디코딩/인코딩 오류
- `ErrorResponse`는 SQL 실행 오류를 PostgreSQL 형식으로 클라이언트에 전달
- 모든 에러는 `Severity` (ERROR/FATAL/PANIC) + `SqlState` (5자리 코드) 조합

## 주의사항

- **Startup 메시지 특수 처리**: 일반 메시지와 헤더 포맷이 다름 (`ConnectionCodec.startup_received` 플래그)
- **채널 통신 에러**: `shared_state.sender.send()` 실패 시 `ErrorResponse::fatal` 반환
- **Portal은 한 번만 fetch 가능**: consume-once 패턴
- **ExecuteField 타입 매핑**: `Bool/Integer/Float/String/Null` → pgwire 데이터 타입으로 변환
