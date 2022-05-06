pub fn get_profile_path() -> String {
    let username = whoami::username();
    let user_path = format!("/Users/{}", username);
    let profile_path = format!("{}/.zshenv", user_path);

    profile_path
}
