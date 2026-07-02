# RRDB Command AGENTS.md

## 개요

`command/` 모듈은 RRDB의 **CLI 인터페이스**를 정의합니다. `clap` derive 매크로로 서브커맨드 파서를 생성합니다.

## CLI 아키텍처

```
rrdb [SUBCOMMAND] [OPTIONS]

SUBCOMMAND:
  init          설정 파일 및 저장공간 초기화
  run           DB 서버 실행
  daemon        데몬 등록 및 실행
  client        DB 클라이언트 실행 (미구현)
```

## Command 구조체

```rust
// src/command/mod.rs
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Command {
    #[clap(subcommand)]
    pub action: SubCommand,
}

#[derive(clap::Subcommand, Debug)]
pub enum SubCommand {
    /// 설정 파일 및 저장공간 초기화
    Init(init::Command),
    /// DB 서버 실행
    Run(run::Command),
    /// 데몬 등록 및 실행
    Daemon(daemon::Command),
    /// DB 클라이언트 실행
    Client,
}
```

## 서브커맨드 상세

### Init — 저장소 초기화

```rust
// src/command/init.rs
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptions {
    #[clap(name = "base-path", long, short)]
    pub base_path: Option<String>,
}

#[derive(Clone, Debug, Args)]
#[clap(name = "init")]
pub struct Command {
    #[clap(flatten)]
    pub init: ConfigOptions,
}
```

**동작**: `DBEngine::initialize_with_base_path()` 호출 → data/wal 디렉토리 생성

```
cargo run -- init
cargo run -- init --base-path /custom/path
```

### Run — 서버 실행

```rust
// src/command/run.rs
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptions {
    #[clap(name = "base-path", long, short)]
    pub base_path: Option<String>,
}

#[derive(Clone, Debug, Args)]
#[clap(name = "run")]
pub struct Command {
    #[clap(flatten)]
    pub value: ConfigOptions,
}
```

**동작**:
1. 설정 로드 (`load_launch_config`)
2. 배너 출력
3. `Server::new(config).run().await` 호출 → TCP 리스너 시작

```
cargo run -- run
cargo run -- run --base-path ./test-data
```

### Daemon — 시스템 데몬 등록

```rust
// src/command/daemon.rs
pub struct Command {}  // 추가 옵션 없음
```

**동작**: `DBEngine::install_daemon()` 호출
- macOS: launchd plist 생성 (`/Library/LaunchDaemons/io.github.myyrakle.rrdb.plist`)
- Linux: systemd 서비스 생성

```
cargo run -- daemon
```

### Client — 대화형 클라이언트 (미구현)

```rust
// SubCommand::Client → println!("Client"); unimplemented!();
```

향후 `psql`-like REPL 구현 예정

## 메인 디스패치 흐름

```rust
// main.rs
#[tokio::main]
async fn main() -> errors::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    let args = Command::parse();

    match args.action {
        SubCommand::Init(init) => {
            let base_path = init.init.base_path.map(PathBuf::from);
            let config = /* load or default */;
            let engine = DBEngine::new(config);
            engine.initialize_with_base_path(base_path).await?;
        }
        SubCommand::Run(run) => {
            let base_path = run.value.base_path.map(PathBuf::from);
            let config = load_launch_config(base_path.as_ref()).await?;
            print_banner();
            let server = Server::new(config);
            server.run().await?;
        }
        SubCommand::Daemon(_) => {
            let config = LaunchConfig::load_from_path(None).await.unwrap_or_default();
            let engine = DBEngine::new(config);
            engine.install_daemon().await?;
        }
        SubCommand::Client => {
            unimplemented!();  // TODO
        }
    }
    Ok(())
}
```

## 새 서브커맨드 추가 방법

1. `src/command/<name>.rs` 파일 추가
2. 해당 파일에 `Command`/`ConfigOptions`를 정의하고, 필요하면 `#[clap(flatten)]`으로 감싼다.
3. `src/command/mod.rs`에 `pub mod <name>;`와 `SubCommand` 변형을 추가한다.
4. `main.rs`의 match에 처리 로직을 추가한다.

```rust
// 1. src/command/dump.rs
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptions {
    #[clap(name = "output-path", long, short)]
    pub output_path: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct Command {
    #[clap(flatten)]
    pub value: ConfigOptions,
}

// 2. src/command/mod.rs
pub mod dump;

// 3. SubCommand enum
#[derive(clap::Subcommand, Debug)]
pub enum SubCommand {
    Init(init::Command),
    Run(run::Command),
    Daemon(daemon::Command),
    Client,
    /// 데이터베이스 덤프
    Dump(dump::Command),  // ← 추가
}

// 4. main.rs
SubCommand::Dump(dump) => {
    // 덤프 로직
}
```

## 주의사항

- **`Command::parse()`**: clap derive 사용, `#[clap]` 속성으로 이름/설명/플래그 설정
- **`SubCommand::Client`는 values 없음**: 단순 variant, 추가 구조체 없음
- **`base_path`는 `Option<String>`**: `PathBuf::from()`으로 변환 후 사용
- **`.init.base_path` 이중 접근**: Init 커맨드가 `init::CommandAttr`을 감싸는 구조일 수 있음 (values를 통한 접근 확인)
- **cargo run 인자 전달**: `--` separator 사용 (`cargo run -- run --base-path ./data`)
