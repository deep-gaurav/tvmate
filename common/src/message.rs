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
    UserJoined(UserJoined),
    UserLeft(UserLeft),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserJoined {
    pub new_user: Uuid,
    pub users: Vec<UserMeta>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserLeft {
    pub user_left: Uuid,
    pub users: Vec<UserMeta>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoomJoinInfo {
    pub room_id: String,
    pub user_id: Uuid,
    pub users: Vec<UserMeta>,
}
