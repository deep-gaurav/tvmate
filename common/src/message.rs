use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Message {
    ServerMessage(ServerMessage),
    ClientMessage,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    RoomCreated(String),
}
