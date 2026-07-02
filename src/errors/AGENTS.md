# RRDB Errors AGENTS.md

## 오류 체계 개요

RRDB는 계층화된 오류 처리 시스템을 사용합니다:

```text
main.rs ── anyhow::Result   (바이너리 레벨)
  │
  ▼
Errors (커스텀) ── pub type Result<T> = std::result::Result<T, Errors>
  │
  ├── ErrorKind::ExecuteError(String)    ← ExecuteError::wrap()
  ├── ErrorKind::LexingError(String)     ← LexingError::wrap()
  ├── ErrorKind::ParsingError(String)    ← ParsingError::wrap()
  ├── ErrorKind::TypeError(String)       ← TypeError::wrap()
  ├── ErrorKind::WALError(String)        ← WALError::wrap()
  ├── ErrorKind::IntoError(String)       ← (미사용/준비)
  └── ErrorKind::ServerError(String)     ← (미사용/준비)
```

## Errors 구조체

```rust
// src/errors.rs
pub struct Errors {
    pub kind: ErrorKind,          // 오류 종류
    pub backtrace: Backtrace,     // 자동 캡처된 백트레이스
    pub message: Option<String>,  // 추가 컨텍스트 메시지
}

impl Errors {
    pub fn new(kind: ErrorKind) -> Self;       // Backtrace 자동 캡처
    pub fn with_message(self, msg: String) -> Self; // 추가 메시지 체이닝
}

// Result 타입 alias
pub type Result<T> = std::result::Result<T, Errors>;
```

## ErrorKind 열거형

```rust
#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    ExecuteError(String),    // SQL 실행 중 오류
    LexingError(String),     // SQL 어휘 분석 오류
    ParsingError(String),    // SQL 구문 분석 오류
    TypeError(String),       // 타입 불일치 오류
    WALError(String),        // WAL 쓰기/읽기 오류
    IntoError(String),       // (준비 중) 변환 오류
    ServerError(String),     // (준비 중) 서버 내부 오류
}
```

## 하위 에러 타입 (thiserror 패턴)

각 하위 에러 타입은 동일한 구조를 가집니다:

```rust
// src/errors/execute_error.rs
pub struct ExecuteError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl ExecuteError {
    // ErrorKind로 감싸서 Errors 반환
    pub fn wrap<T: ToString>(message: T) -> Errors {
        Errors::new(ErrorKind::ExecuteError(message.to_string()))
    }
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
```

하위 에러 타입들:

| 타입 | Display 형식 | ErrorKind 매핑 |
|---|---|---|
| `ExecuteError` | `"{message}"` | `ErrorKind::ExecuteError` |
| `LexingError` | `"lexing error: {message}"` | `ErrorKind::LexingError` |
| `ParsingError` | `"parsing error: {message}"` | `ErrorKind::ParsingError` |
| `TypeError` | `"type error: {message}"` | `ErrorKind::TypeError` |
| `WALError` | `"wal error: {message}"` | `ErrorKind::WALError` |

## 사용 패턴

### 라이브러리 내부 (engine/errors/pgwire)

```rust
// engine actions에서 ExecuteError 사용
use crate::errors::execute_error::ExecuteError;

async fn create_table(&self, query: CreateTableQuery) -> errors::Result<ExecuteResult> {
    // 성공 시 Ok
    // 실패 시:
    Err(ExecuteError::wrap("table already exists"))
    // 또는
    Err(ExecuteError::wrap(format!("io error: {}", e)))
}

// lexer에서 LexingError 사용
Err(LexingError::wrap("unexpected character"))
```

### 바이너리 레벨 (main.rs)

```rust
// main 함수는 errors::Result 사용 (Errors 타입)
#[tokio::main]
async fn main() -> errors::Result<()> {
    // ...
    Ok(())
}

// config 로딩 등 외부 의존성은 anyhow::Result
use anyhow;
impl LaunchConfig {
    pub async fn load_from_path(filepath: Option<String>) -> anyhow::Result<Self> {
        let content = tokio::fs::read_to_string(filepath).await?;
        // ? 연산자로 anyhow::Error 자동 변환
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
```

### pgwire에서의 에러 처리

```rust
// ProtocolError: thiserror enum (별도 체계)
#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("parsing error")]
    ParserError,
    #[error("invalid message type: {0}")]
    InvalidMessageType(u8),
    #[error("invalid format code: {0}")]
    InvalidFormatCode(i16),
}

// SQL 실행 오류는 ErrorResponse로 클라이언트에 전송
ErrorResponse::fatal(SqlState::CONNECTION_EXCEPTION, "message")
ErrorResponse::error(SqlState::SYNTAX_ERROR, "message")
```

## Display / Debug 구현

```rust
// Display: 사람이 읽기 쉬운 메시지
impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(msg) = &self.message {
            write!(f, "{}: {}", self.kind, msg)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}

// Debug: 에러 메시지 + 백트레이스 전체 출력
impl fmt::Debug for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(msg) = &self.message {
            write!(f, "{:?} = {}\n{}", self.kind, msg, self.backtrace)
        } else {
            write!(f, "{:?}\n{}", self.kind, self.backtrace)
        }
    }
}
```

## 주의사항

- **`Errors`는 `std::error::Error` 구현**: `?` 연산자와 호환
- **`PartialEq` for 하위 타입**: message 필드만 비교 (backtrace 무시)
- **`ErrorKind`의 `PartialEq`**: derive 사용, 변형별 문자열 비교
- **`Backtrace` 캡처**: `Errors::new()` 호출 시 자동 캡처 (`std::backtrace::Backtrace::capture()`)
- **추가 메시지**: `Errors::with_message()`로 상위 컨텍스트 추가 가능
- **`ProtocolError`는 pgwire 모듈 전용**: thiserror derive로 wire 레벨 오류 처리
- 새 에러 타입 추가 시: (1) `errors/<name>.rs` 생성, (2) `errors.rs`에 mod 선언 + `ErrorKind` 변형 추가
