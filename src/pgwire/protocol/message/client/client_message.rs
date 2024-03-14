use crate::lib::pgwire::protocol::{Bind, Describe, Execute, Parse, Startup};

#[derive(Debug)]
pub enum ClientMessage {
    SSLRequest, // for SSL negotiation
    Startup(Startup),
    Parse(Parse),
    Describe(Describe),
    Bind(Bind),
    Sync,
    Execute(Execute),
    Query(String),
    Terminate,
}
