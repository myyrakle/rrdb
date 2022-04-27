pub struct ServerOption {
    pub port: u32,
    pub host: String,
}

impl std::default::Default for ServerOption {
    fn default() -> Self {
        Self {
            port: 55555,
            host: "0.0.0.0".into(),
        }
    }
}
