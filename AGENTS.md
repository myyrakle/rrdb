# RRDB (Rust RDB) AGENTS.md

## 프로젝트 개요

RRDB는 **Rust**로 작성된 **PostgreSQL 호환 관계형 데이터베이스**입니다. "Rust RDB"의 약자로, 한국 개발자(myyrakle)가 주도합니다. PostgreSQL wire protocol을 구현하여 표준 `psql` 클라이언트 및 PostgreSQL 라이브러리와 호환됩니다.

- 저장소: <https://github.com/myyrakle/rrdb>
- 라이선스: MIT
- Rust Edition: 2024

## 기술 스택

| 구성 요소 | 사용 기술 |
|---|---|
| 언어/에디션 | **Rust 2024 edition** |
| 비동기 런타임 | **Tokio** (features = `["full"]`) — async I/O, 타이머, 채널, 파일시스템 전반 |
| CLI 프레임워크 | **clap** 3.x (derive 매크로) |
| 설정 파일 | **serde** + **toml** — `LaunchConfig`를 TOML에서 역직렬화 |
| WAL 인코딩 | **bincode** — 바이너리 직렬화로 WAL 세그먼트 저장 |
| Wire Protocol | **tokio-util** codec — PostgreSQL pgwire Decoder/Encoder |
| 오류 처리 | **thiserror** (라이브러리 에러) + **anyhow** (바이너리 레벨) |
| 로깅 | **env_logger** + **log** — `RUST_LOG` 환경변수로 레벨 제어 |
| 테스트 | **mockall** (모킹), `#[tokio::test]` (비동기 테스트) |

## 아키텍처 다이어그램

```text
┌────────────────────────────────────────────────────────────┐
│                      main.rs (entry)                        │
│  #[tokio::main]  ──  clap::Parser::parse()                  │
└────────┬───────────────────────┬───────────────────────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐   ┌─────────────────────┐
│  command/        │   │  config/             │
│  SubCommand      │   │  LaunchConfig        │
│  ├─ Init         │   │  (TOML → serde)      │
│  ├─ Run          │   └──────────┬──────────┘
│  ├─ Daemon       │              │
│  └─ Client       │              │ Arc<LaunchConfig>
└────────┬─────────┘              │
         │                        │
         ▼                        ▼
┌─────────────────────────────────────────────────┐
│                engine/                            │
│  SQL text → lexer → parser → AST → optimizer      │
│         → actions (DDL/DML/Other)                  │
│                                                   │
│  ├─ server/  (TCP listener, channel dispatch)     │
│  ├─ wal/     (WAL manager + bincode encoder)      │
│  └─ schema/  (table/config storage)               │
└──────────────────┬──────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────┐
│                pgwire/                            │
│  PostgreSQL wire protocol v3                      │
│  ├─ protocol/  (codec, messages, types)           │
│  ├─ engine/    (Engine trait → RRDBEngine)        │
│  ├─ connection/ (per-client TCP connection)       │
│  └─ predule/   (pgwire 공용 타입)                  │
└─────────────────────────────────────────────────┘
```

## 모듈 책임

### `command/` — CLI 서브커맨드
- `init`: 설정 파일 및 데이터 디렉토리 초기화
- `run`: DB 서버 실행 (TCP 리스닝)
- `daemon`: 시스템 데몬 등록 (launchd / systemd)
- `client`: (미구현) 대화형 SQL 클라이언트

### `common/` — 공통 트레이트
- `FileSystem`: 파일/디렉토리 생성 추상화 (실제 구현: `tokio::fs`)
- `CommandRunner`: 외부 명령어 실행 추상화 (실제 구현: `std::process::Command`)
- `Arc<dyn Trait + Send + Sync>`로 의존성 주입, 테스트 시 mockall로 대체

### `config/` — 설정 관리
- `LaunchConfig`: 포트, 호스트, 데이터 경로, WAL 설정을 포함
- TOML 파일 로드 → serde 역직렬화 → `Arc<LaunchConfig>`로 공유

### `constants/` — 상수
- 기본 데이터베이스명(`rrdb`), 설정파일명(`rrdb.config`), OS별 기본 경로
- launchd/systemd 데몬 스크립트 템플릿

### `engine/` — DB 엔진 코어
- SQL 텍스트 → lexer → parser(→ AST) → optimizer → actions 실행
- `DBEngine`: 쿼리 프로세싱, WAL 연동, 스키마 관리
- 상세: [src/engine/AGENTS.md](src/engine/AGENTS.md)

### `pgwire/` — PostgreSQL Wire Protocol
- TCP 연결 처리, PostgreSQL 메시지 코덱, 쿼리 디스패치
- 상세: [src/pgwire/AGENTS.md](src/pgwire/AGENTS.md)

### `errors/` — 오류 체계
- `Errors` (커스텀), `ErrorKind` 열거형, 6개 하위 에러 타입
- 상세: [src/errors/AGENTS.md](src/errors/AGENTS.md)

### `utils/` — 유틸리티
- `collection`, `float`, `macos`, `predule`

## 코딩 컨벤션

### 에러 처리

```rust
// 라이브러리 에러: thiserror (ExecuteError, LexingError 등)
// 각 에러 타입은 ErrorKind로 매핑
ExecuteError::wrap("message")       // → Errors(ErrorKind::ExecuteError)

// 커스텀 Errors 타입을 Result로 사용
pub type Result<T> = std::result::Result<T, Errors>;
// main.rs 등 바이너리 레벨: anyhow
pub async fn load_from_path(...) -> anyhow::Result<Self>;
```

### 비동기

```rust
// 모든 I/O 함수는 async fn
async fn create_dir(&self, path: &str) -> io::Result<()>;
// tokio::fs 사용 (동기 fs 대신)
tokio::fs::read(config_path).await?;
// 비동기 테스트
#[tokio::test]
async fn test_name() { ... }
```

### 설정 공유

```rust
// 설정은 Arc로 공유
let config: Arc<LaunchConfig>;
pub struct DBEngine {
    pub(crate) config: Arc<LaunchConfig>,
    // ...
}
```

### 로깅

```rust
// env_logger 기본값 info 레벨
// 실행 시: RUST_LOG=debug cargo run -- run
log::info!("data path: {}", config.data_directory);
log::debug!("AST echo: {:?}", statement);
```

### 테스트

```rust
// mockall로 외부 의존성 모킹
#[mockall::automock]
pub trait FileSystem { ... }

// 비동기 테스트
#[tokio::test]
async fn test_process_query() { ... }
```

## 빌드/테스트

```bash
# 전체 빌드
cargo build

# 테스트 실행
cargo test

# 서버 실행 (기본 설정)
cargo run -- run

# 서버 실행 (커스텀 경로)
cargo run -- run --base-path /path/to/data

# debug 로깅
RUST_LOG=debug cargo run -- run

# 초기화
cargo run -- init
```

### Features

| feature | 설명 |
|---|---|
| `rrdb` (default) | 메인 바이너리 빌드, `cli` 포함 |
| `cli` | `atty`, `structopt` 의존성 활성화 |

## PR 규칙

| 브랜치 패턴 | 용도 |
|---|---|
| `feat/#issue-short-description` | 기능 개발 |
| `fix/#issue-short-description` | 버그 수정 |
| `test/#issue-short-description` | 테스트 추가 |
| `refactor/#issue-short-description` | 리팩토링 |
| `chore/#issue-short-description` | 빌드/설정/CI 변경 |

- PR은 `master` 브랜치로 머지
- clippy 린트 통과 필수 (lint `to_string_trait_impl` 허용됨)

## 주의사항

### `PathBuf` 처리
- `config.data_directory`는 `String` 타입 → 사용 시 `PathBuf::from()`으로 변환
- `with_base_path()` 호출 시 `absolute_path()`로 절대경로 보장
- 경로 결합 시 `PathBuf::join()` 사용, 문자열 concat 금지

### `Arc` 공유 상태
- `LaunchConfig`는 `Clone + Serialize + Deserialize` — `Arc<LaunchConfig>`로 엔진/서버 전반에서 공유
- `SharedState` (engine/server/)는 `tokio::sync::oneshot` 채널로 통신

### WAL 세그먼트 관리
- 기본 세그먼트 크기: 16MB (`1024 * 1024 * 16`)
- WAL 확장자: `log` (기본값)
- 인코더: `BincodeEncoder` (`bincode` 크레이트)
- WAL 설정 필드: `wal_enabled`, `wal_directory`, `wal_segment_size`, `wal_extension`

### OS별 경로
- Linux/macOS: `/var/lib/rrdb` (기본)
- Windows: `C:\Program Files\rrdb` (기본)
- macOS 데몬: `launchd` → `/Library/LaunchDaemons/io.github.myyrakle.rrdb.plist`
- Linux 데몬: `systemd` → `/etc/systemd/system/rrdb.service`
