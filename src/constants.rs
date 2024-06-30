// 기본 데이터베이스 이름
pub const DEFAULT_DATABASE_NAME: &str = "rrdb";

// 기본 설정파일 이름.
pub const DEFAULT_CONFIG_FILENAME: &str = "rrdb.config";

// 기본 Data 디렉터리 이름
pub const DEFAULT_DATA_DIRNAME: &str = "data";

// 운영체제별 기본 저장 경로를 반환합니다.
#[cfg(target_os = "linux")]
pub const DEFAULT_CONFIG_BASEPATH: &str = "/var/lib/rrdb";

#[cfg(target_os = "windows")]
pub const DEFAULT_CONFIG_BASEPATH: &str = r"C:\Program Files\rrdb";

#[cfg(target_os = "macos")]
pub const DEFAULT_CONFIG_BASEPATH: &str = "/var/lib/rrdb";

pub const LAUNCHD_PLIST_PATH: &str = "/Library/LaunchDaemons/io.github.myyrakle.rrdb.plist";
