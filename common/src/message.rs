use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::UserMeta;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Message {
    ServerMessage(ServerMessage),
    ClientMessage,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    RoomCreated(RoomJoinInfo),
    RoomJoined(RoomJoinInfo),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoomJoinInfo {
    pub room_id: String,
    pub user_id: Uuid,
    pub users: Vec<UserMeta>,
}
