use winreg::{enums::*, RegKey};

pub fn get_system_env<S: std::string::ToString>(key: S) -> String {
    let key = key.to_string();

    if cfg!(windows) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (env, _) = hkcu.create_subkey("Environment").unwrap(); // create_subkey opens with write permissions
        let value: String = env.get_value(key.as_str()).unwrap();

        value
    } else if cfg!(linux) {
        // TODO:
        "".into()
    } else if cfg!(macos) {
        // TODO:
        "".into()
    } else {
        "".into()
    }
}

pub fn set_system_env<S: std::string::ToString>(key: S, value: S) {
    let key = key.to_string();
    let value = value.to_string();

    if cfg!(windows) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (env, _) = hkcu.create_subkey("Environment").unwrap(); // create_subkey opens with write permissions
        env.set_value(&key, &value).unwrap();
    } else if cfg!(linux) {
        // TODO:
    } else if cfg!(macos) {
        // TODO:
    }
}
