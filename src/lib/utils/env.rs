use std::process::Command;
use std::{fs, io::Write};

use super::get_profile_path;

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
        "fo".to_string()
    }

    #[cfg(target_os = "macos")]
    {
        std::env::var(key).unwrap()
    }
}

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
    {}

    #[cfg(target_os = "macos")]
    {
        let profile_path = get_profile_path();

        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(profile_path)
            .unwrap();

        let export_line = format!("export {}={}\n", key, value);

        file.write_all(export_line.as_bytes()).unwrap();
        std::env::set_var(key, value);
        // Command::new("export")
        //     .arg(format!("{}={}", key, value))
        //     .spawn()
        //     .unwrap();
    }
}
