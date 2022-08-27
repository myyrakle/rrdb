use std::collections::HashMap;

#[derive(Debug)]
pub struct Startup {
    pub requested_protocol_version: (i16, i16),
    pub parameters: HashMap<String, String>,
}
