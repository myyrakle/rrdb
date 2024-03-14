// 운영 체제에 종속적인 형태로, 파일 저장경로 등에 대한 값을 환경변수로 저장합니다.
// Windows, Linux, MacOS를 위주로 지원합니다.

// 환경변수를 가져옵니다.
pub fn get_system_env<S: std::string::ToString>(key: S) -> String {
    let key = key.to_string();

    #[cfg(target_os = "windows")]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (env, _) = hkcu.create_subkey("Environment").unwrap(); // create_subkey opens with write permissions
        let value: String = env.get_value(key.as_str()).unwrap();

        value
    }

    #[cfg(target_os = "linux")]
    {
        // use super::get_profile_path;
        // use std::process::Command;
        // use std::{fs, io::Write};
        "fo".to_string()
    }

    #[cfg(target_os = "macos")]
    {
        // use super::get_profile_path;
        // use std::process::Command;
        // use std::{fs, io::Write};
        std::env::var(key).unwrap()
    }
}

// 환경변수를 설정합니다.
pub fn set_system_env<S: std::string::ToString>(key: S, value: S) {
    let key = key.to_string();
    let value = value.to_string();

    #[cfg(target_os = "windows")]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (env, _) = hkcu.create_subkey("Environment").unwrap(); // create_subkey opens with write permissions
        env.set_value(&key, &value).unwrap();
    }

    #[cfg(target_os = "linux")]
    {
        // use super::get_profile_path;
        // use std::process::Command;
        // use std::{fs, io::Write};
    }

    #[cfg(target_os = "macos")]
    {
        use super::get_profile_path;
        use std::process::Command;
        use std::{fs, io::Write};

        let profile_path = get_profile_path();

        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(profile_path)
            .unwrap();

        let export_line = format!("export {}={}\n", key, value);

        file.write_all(export_line.as_bytes()).unwrap();
        std::env::set_var(key, value);
        Command::new("export FOO=\"BAR\"").output().unwrap();
        // Command::new("export")
        //     .arg(format!("{}={}", key, value))
        //     .spawn()
        //     .unwrap();
    }
}
