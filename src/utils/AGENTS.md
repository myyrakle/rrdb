# RRDB Utils AGENTS.md

## 개요

`utils/` 모듈은 RRDB 전반에서 사용되는 범용 유틸리티 함수들을 제공합니다. 컬렉션 연산, 부동소수점 처리, macOS 특화 코드, 공용 임포트(Prelude)를 포함합니다.

## 모듈 구성

```
utils/
├── mod.rs        (pub mod collection; pub mod float; pub mod macos; pub mod predule;)
├── collection.rs  — 컬렉션 관련 헬퍼
├── float.rs       — 부동소수점 유틸리티
├── macos.rs       — macOS 전용 코드
└── predule.rs     — 공용 임포트/타입 모음
```

## collection.rs

컬렉션 타입 관련 헬퍼 함수들:

```rust
// (구현에 따라 함수 시그니처 상이)
// 예상 기능:
// - Vec<T> 정렬/필터링 헬퍼
// - HashMap 병합
// - Option<T> 조합 유틸리티
```

## float.rs

부동소수점 비교 및 변환 유틸리티:

```rust
// (구현에 따라 함수 시그니처 상이)
// 예상 기능:
// - f64/f32 근사 비교 (epsilon)
// - NaN/Infinity 처리
// - 문자열 파싱
```

## macos.rs

macOS 플랫폼 전용 코드:

```rust
// (구현에 따라 함수 시그니처 상이)
// 예상 기능:
// - macOS 경로 처리 (/var/lib/rrdb 등)
// - launchd 통합 관련 유틸리티
// - macOS 특화 파일시스템 작업
```

## predule.rs

전역에서 자주 사용되는 타입/트레이트/함수의 **Prelude** 모듈:

```rust
// utils/predule.rs
// 용도: 자주 사용하는 임포트를 한 곳에 모아 re-export
//
// pub use ...
// 예: pub use crate::errors::Errors;
//     pub use std::sync::Arc;
```

다른 모듈에서 `use crate::utils::predule::*;`로 간편하게 임포트할 수 있습니다.

## 사용 패턴

```rust
// utils 모듈 사용 예시
use crate::utils::predule::*;

// collection 헬퍼
use crate::utils::collection;

// float 비교
if utils::float::approx_eq(a, b) { ... }
```

## 새 유틸리티 추가 방법

1. `src/utils/<name>.rs` 파일 생성
2. 함수/상수 정의
3. `src/utils/mod.rs`에 `pub mod <name>;` 추가
4. 필요시 `predule.rs`에 re-export 추가

```rust
// 예시: src/utils/string.rs 새로 추가
pub fn trim_all(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// src/utils/mod.rs
pub mod string;

// src/utils/predule.rs
pub use super::string;
```

## 주의사항

- **`predule.rs`는 과도한 re-export 금지**: 프로젝트 전반에서 널리 사용되는 타입만 포함
- **macOS 코드는 `#[cfg(target_os = "macos")]`**: 다른 플랫폼에서 컴파일되지 않아야 함
- **컬렉션 유틸리티**: `itertools` 크레이트가 이미 의존성에 있으므로 중복 구현 피하기
- **float 비교**: Rust의 기본 `f64::EPSILON` 또는 직접 epsilon 정의 사용
- **모듈 간 의존성**: utils 모듈은 다른 내부 모듈에 의존하지 않아야 함 (가장 하위 레벨)
