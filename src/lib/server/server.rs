use crate::lib::server::ServerOption;

pub struct Server {
    pub option: ServerOption,
}

impl Server {
    pub fn new(option: ServerOption) -> Self {
        Self { option }
    }

    pub async fn run(&self) {
        println!("# Server is Running at {}", self.option.port);

        loop {}
    }
}
