use tokio::sync::mpsc::Sender;

use super::{channel::ChannelRequest, client::ClientInfo};

#[derive(Clone, Debug)]
pub struct SharedState {
    pub sender: Sender<ChannelRequest>,
    pub client_info: ClientInfo,
}
