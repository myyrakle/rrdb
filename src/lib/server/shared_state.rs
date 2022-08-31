use tokio::sync::mpsc::Sender;

use super::channel::ChannelRequest;

pub struct SharedState {
    pub sender: Sender<ChannelRequest>,
}
