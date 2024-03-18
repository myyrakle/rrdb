// 운영 체제에 종속적인 형태로, 파일 저장경로 등에 대한 값을 환경변수로 저장합니다.
// Windows, Linux, MacOS를 위주로 지원합니다.

use std::{path::PathBuf, str::FromStr};

// 운영체제별 기본 저장 경로를 반환합니다.
// 현재는 Windows, Linux만 지원합니다.
#[allow(unreachable_code)]
pub fn get_target_basepath() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        return PathBuf::from_str("C:\\Program Files\\rrdb").unwrap();
    }

    #[cfg(target_os = "linux")]
    {
        return PathBuf::from_str("/var/lib/rrdb").unwrap();
    }

    // #[cfg(target_os = "macos")]
    // {
    // }

    unimplemented!("Not supported OS");
}
