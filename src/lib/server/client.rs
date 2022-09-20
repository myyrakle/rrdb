use std::net::IpAddr;

#[derive(Clone, Debug)]
pub struct ClientInfo {
    pub ip: IpAddr,
    pub connection_id: String,
    pub database: String,
}
