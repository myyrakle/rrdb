# RRDB Common AGENTS.md

## 개요

`common/` 모듈은 RRDB 전반에서 사용되는 **공통 트레이트**와 **실제 구현체**를 제공합니다. 의존성 역전 원칙(DIP)에 따라 추상화된 인터페이스와 테스트용 목 구현을 함께 정의합니다.

## 모듈 구성

```
common/
├── mod.rs   (pub mod command; pub mod fs;)
├── fs.rs        (FileSystem trait + RealFileSystem)
└── command.rs   (CommandRunner trait + RealCommandRunner)
```

## FileSystem 트레이트

```rust
// src/common/fs.rs
use futures::io;

#[mockall::automock]
#[async_trait::async_trait]
pub trait FileSystem {
    async fn create_dir(&self, path: &str) -> io::Result<()>;
    async fn write_file(&self, path: &str, content: &[u8]) -> io::Result<()>;
}
```

### 실제 구현: `RealFileSystem`

```rust
pub struct RealFileSystem;

#[async_trait::async_trait]
impl FileSystem for RealFileSystem {
    async fn create_dir(&self, path: &str) -> io::Result<()> {
        tokio::fs::create_dir(path).await
    }

    async fn write_file(&self, path: &str, content: &[u8]) -> io::Result<()> {
        tokio::fs::write(path, content).await
    }
}
```

## CommandRunner 트레이트

```rust
// src/common/command.rs
use std::{io, process::{Command, Output}};

#[mockall::automock]
pub trait CommandRunner {
    fn run(&self, command: &mut Command) -> io::Result<Output>;
}
```

### 실제 구현: `RealCommandRunner`

```rust
pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, command: &mut Command) -> io::Result<Output> {
        command.output()  // 동기 시스템 명령어 실행
    }
}
```

## 의존성 주입 패턴

```rust
use std::sync::Arc;
use crate::{common::{fs::FileSystem, command::CommandRunner}, config::launch_config::LaunchConfig};

pub struct DBEngine {
    pub(crate) config: Arc<LaunchConfig>,
    pub(crate) file_system: Arc<dyn FileSystem + Send + Sync>,
    pub(crate) command_runner: Arc<dyn CommandRunner + Send + Sync>,
}

impl DBEngine {
    pub fn new(config: LaunchConfig) -> Self {
        Self {
            config: Arc::new(config),
            file_system: Arc::new(RealFileSystem {}),
            command_runner: Arc::new(RealCommandRunner {}),
        }
    }
}
```

## 테스트에서의 목 사용

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::fs::MockFileSystem;

    #[tokio::test]
    async fn test_create_dir_calls_fs() {
        let mut mock_fs = MockFileSystem::new();
        mock_fs.expect_create_dir()
            .with(predicate::eq("/tmp/test"))
            .times(1)
            .returning(|_| Ok(()));

        let engine = DBEngine {
            config: Arc::new(LaunchConfig::default()),
            file_system: Arc::new(mock_fs),
            command_runner: Arc::new(RealCommandRunner {}),
        };

        // engine으로 create_table 등 테스트
    }
}
```

## 트레이트 확장 가이드

새로운 common 트레이트를 추가할 때:

1. `src/common/<name>.rs` 파일 생성
2. `#[mockall::automock]` 속성 추가
3. `#[async_trait::async_trait]` (비동기 메서드가 있을 경우)
4. `trait` 정의
5. 실제 구현 `struct*Real` + impl
6. `src/common/mod.rs`에 `pub mod <name>;` 추가
7. `Arc<dyn Trait + Send + Sync>`로 사용

## 주의사항

- **`FileSystem`은 비동기**: `#[async_trait]` 사용, `tokio::fs`로 구현
- **`CommandRunner`는 동기**: `std::process::Command.output()`은 동기 blocking 호출
- **`mockall`로 생성되는 목 타입**: `MockFileSystem`, `MockCommandRunner`
- **`Send + Sync` 필요**: 모든 트레이트 바운드에 포함하여 Arc로 안전하게 공유
- **의존성 주입은 생성자에서만**: `DBEngine::new()`가 Real 구현체를 기본 주입
