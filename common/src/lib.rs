pub mod endpoints;
pub mod message;
pub mod message_sender;
pub mod params;
pub mod util;

use message::Message;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "ssr")]
pub use crate::ssr::*;

pub struct User {
    pub meta: UserMeta,
    #[cfg(feature = "ssr")]
    pub sender: tokio::sync::mpsc::Sender<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMeta {
    pub id: Uuid,
    pub name: String,
}

pub struct Room {
    pub users: Vec<User>,
}

#[cfg(feature = "ssr")]
mod ssr {
    use message::RoomJoinInfo;
    use thiserror::Error;
    use tokio::sync::RwLock;
    use util::generate_random_string;

    use super::*;
    use std::{collections::HashMap, sync::Arc};

    #[derive(Clone)]
    pub struct RoomProvider {
        rooms: Arc<RwLock<HashMap<String, Room>>>,
    }

    #[derive(Error, Debug)]
    pub enum RoomProviderError {
        #[error("cannot generate new key")]
        KeyGenerationFailed,
        #[error("given room does not exist")]
        RoomDoesntExist,
    }

    impl RoomProvider {
        pub fn new() -> Self {
            Self {
                rooms: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub async fn new_room(&self, user: User) -> Result<RoomJoinInfo, RoomProviderError> {
            let mut rooms = self.rooms.write().await;
            let id = {
                let mut tries = 5;
                loop {
                    let id = generate_random_string(6);
                    if !rooms.contains_key(&id) {
                        break id;
                    }
                    tries -= 1;
                    if tries <= 0 {
                        return Err(RoomProviderError::KeyGenerationFailed);
                    }
                }
            };
            let user_meta = user.meta.clone();
            rooms.insert(id.clone(), Room::new(user));
            Ok(RoomJoinInfo {
                room_id: id,
                user_id: user_meta.id,
                users: vec![user_meta],
            })
        }

        pub async fn join_room(
            &self,
            room_id: &str,
            user: User,
        ) -> Result<RoomJoinInfo, RoomProviderError> {
            let mut rooms = self.rooms.write().await;
            let user_id = user.meta.id.clone();
            if let Some(room) = rooms.get_mut(room_id) {
                room.users.push(user);
                Ok(RoomJoinInfo {
                    room_id: room_id.to_string(),
                    user_id,
                    users: room.users.iter().map(|u| u.meta.clone()).collect(),
                })
            } else {
                Err(RoomProviderError::RoomDoesntExist)
            }
        }

        pub async fn remove_user(&self, room_id: &str, user_id: Uuid) {
            let mut rooms = self.rooms.write().await;
            if let Some(room) = rooms.get_mut(room_id) {
                room.users.retain(|user| user.meta.id == user_id);
                if room.users.is_empty() {
                    rooms.remove(room_id);
                }
            }
        }
    }

    impl Room {
        pub fn new(user: User) -> Self {
            Self { users: vec![user] }
        }
    }
}
