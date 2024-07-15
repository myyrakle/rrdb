// MacOS에서 고정 환경변수를 가져옵니다.
pub fn get_profile_path() -> String {
    let username = whoami::username();
    let user_path = format!("/Users/{}", username);
    let profile_path = format!("{}/.zshenv", user_path);

    profile_path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_profile_path() {
        let username = whoami::username();
        let user_path = format!("/Users/{}", username);
        let profile_path = format!("{}/.zshenv", user_path);

        assert_eq!(get_profile_path(), profile_path);
    }
}
