pub struct ServerOption {
    pub port: u32,
}

impl std::default::Default for ServerOption {
    fn default() -> Self {
        Self { port: 55555 }
    }
}
