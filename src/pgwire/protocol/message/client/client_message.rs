use super::{Bind, Close, Describe, Execute, Parse, Startup};

#[derive(Debug)]
pub enum ClientMessage {
    SSLRequest, // for SSL negotiation
    Startup(Startup),
    Parse(Parse),
    Describe(Describe),
    Close(Close),
    Bind(Bind),
    Sync,
    Execute(Execute),
    Query(String),
    Terminate,
}
