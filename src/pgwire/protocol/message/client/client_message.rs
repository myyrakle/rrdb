use super::{Bind, Close, Describe, Execute, Parse, Startup};

#[derive(Debug)]
pub enum ClientMessage {
    SSLRequest, // for SSL negotiation
    GSSENCRequest,
    Startup(Startup),
    Parse(Parse),
    Describe(Describe),
    Close(Close),
    Bind(Bind),
    Flush,
    Sync,
    Execute(Execute),
    Query(String),
    Terminate,
}
