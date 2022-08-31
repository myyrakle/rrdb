use tokio::sync::mpsc::Sender;

use super::channel::ChannelRequest;

#[derive(Clone, Debug)]
pub struct SharedState {
    pub sender: Sender<ChannelRequest>,
}
