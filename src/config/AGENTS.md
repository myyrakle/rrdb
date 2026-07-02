# RRDB Config AGENTS.md

## 개요

`config/` 모듈은 RRDB의 런타임 설정을 관리합니다. TOML 파일에서 설정을 로드하고 `Arc<LaunchConfig>`로 전체 시스템에 공유합니다.

## LaunchConfig 구조체

```rust
// src/config/launch_config.rs
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LaunchConfig {
    pub port: u32,               // 리스닝 포트 (기본: 22208)
    pub host: String,            // 바인딩 호스트 (기본: "0.0.0.0")
    pub data_directory: String,  // 데이터 저장 경로

    // WAL (Write-Ahead Logging)
    pub wal_enabled: bool,       // WAL 활성화 여부 (기본: true)
    pub wal_directory: String,   // WAL 세그먼트 저장 경로
    pub wal_segment_size: u32,   // 세그먼트 최대 크기 (기본: 16MB = 1024*1024*16)
    pub wal_extension: String,   // WAL 파일 확장자 (기본: "log")
}
```

### 기본값

```rust
impl Default for LaunchConfig {
    fn default() -> Self {
        let base_path = PathBuf::from(DEFAULT_CONFIG_BASEPATH);
        Self {
            port: 22208,
            host: "0.0.0.0".to_string(),
            data_directory: base_path.join("data").to_string_lossy().to_string(),
            wal_enabled: true,
            wal_directory: base_path.join("wal").to_string_lossy().to_string(),
            wal_segment_size: 1024 * 1024 * 16,  // 16MB
            wal_extension: "log".to_string(),
        }
    }
}
```

## TOML 설정 파일 형식

기본 설정 파일명: `rrdb.config`

```toml
port = 22208
host = "0.0.0.0"
data_directory = "/var/lib/rrdb/data"
wal_enabled = true
wal_directory = "/var/lib/rrdb/wal"
wal_segment_size = 16777216
wal_extension = "log"
```

## 설정 로딩 흐름

```rust
impl LaunchConfig {
    // 파일 경로에서 TOML 로드 (anyhow::Result, 파일시스템 I/O)
    pub async fn load_from_path(filepath: Option<String>) -> anyhow::Result<Self> {
        let filepath = match filepath {
            Some(path) => PathBuf::from(path),
            None => Self::default_config_path(),  // DEFAULT_CONFIG_BASEPATH / rrdb.config
        };
        let config_str = tokio::fs::read_to_string(filepath).await?;
        let config: Self = toml::from_str(&config_str)?;
        Ok(config)
    }

    // 기본 경로 생성
    pub fn default_config_path() -> PathBuf {
        PathBuf::from(DEFAULT_CONFIG_BASEPATH).join(DEFAULT_CONFIG_FILENAME)
    }

    // base_path 기반 data/wal 디렉토리 재설정
    pub fn with_base_path(mut self, base_path: impl Into<PathBuf>) -> Self {
        let base_path = absolute_path(base_path.into());
        self.data_directory = base_path.join(DEFAULT_DATA_DIRNAME).to_string_lossy().to_string();
        self.wal_directory = base_path.join(DEFAULT_WAL_DIRNAME).to_string_lossy().to_string();
        self
    }

    // base_path 기반 기본 설정 (with_base_path 호출 편의)
    pub fn default_for_base_path(base_path: impl Into<PathBuf>) -> Self {
        Self::default().with_base_path(base_path)
    }
}
```

## Arc 공유 패턴

```rust
use std::sync::Arc;
use crate::config::launch_config::LaunchConfig;

// 1. LaunchConfig 로드 (기본값 또는 파일)
let config = LaunchConfig::load_from_path(None).await.unwrap_or_default();

// 2. Arc로 래핑
let config = Arc::new(config);

// 3. 전체 시스템에 공유
// DBEngine (모듈 전체)
let engine = DBEngine::new_with_arc_config(config.clone());

// Server (TCP 리스너)
let server = Server::new_with_arc_config(config.clone());
```

## main.rs에서의 설정 흐름

```
main()
  │
  ├── SubCommand::Init
  │     ├── base_path 제공 시: load_launch_config(Some(&base_path))
  │     └── base_path 미제공 시: LaunchConfig::default_for_base_path(base_path)
  │
  ├── SubCommand::Run
  │     └── load_launch_config(base_path.as_ref()) → config.wal_directory 로깅
  │
  └── SubCommand::Daemon
        └── LaunchConfig::load_from_path(None).await.unwrap_or_default()
```

```rust
// load_launch_config 도우미 함수 (main.rs)
async fn load_launch_config(base_path: Option<&PathBuf>) -> errors::Result<LaunchConfig> {
    match base_path {
        Some(base_path) => {
            let config_path = base_path.join(DEFAULT_CONFIG_FILENAME);
            let config = LaunchConfig::load_from_path(
                Some(config_path.to_string_lossy().to_string())
            ).await.map_err(|e| ExecuteError::wrap(format!("config load error: {}", e)))?;
            Ok(config.with_base_path(base_path))
        }
        None => LaunchConfig::load_from_path(None)
            .await
            .map_err(|e| ExecuteError::wrap(format!("config load error: {}", e))),
    }
}
```

## 주의사항

- **`data_directory`/`wal_directory`는 `String`**: `PathBuf::from()`으로 변환 필요
- **`with_base_path()`는 상대 경로를 절대 경로로 변환**: `absolute_path()` 내부 호출
- **TOML 필수 필드**: 모든 필드는 TOML에 있어야 함 (기본값이 아닌 required 필드)
- **팔요시 `default()` 사용**: 로딩 실패 시 `unwrap_or_default()`가 안전한 대안
- **Arc Clone 경량화**: `LaunchConfig`의 `Clone` derive로 Arc::clone은 포인터만 복사
