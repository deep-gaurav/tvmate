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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserState {
    VideoNotSelected,
    VideoSelected(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerStatus {
    Paused(f64),
    Playing(f64),
}

impl PlayerStatus {
    /// Returns `true` if the player status is [`Paused`].
    ///
    /// [`Paused`]: PlayerStatus::Paused
    #[must_use]
    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused(..))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMeta {
    pub id: Uuid,
    pub name: String,
    pub state: UserState,
}

pub struct Room {
    pub users: Vec<User>,
    pub player_status: PlayerStatus,
}

#[cfg(feature = "ssr")]
mod ssr {
    use futures::{stream::FuturesUnordered, StreamExt, TryStreamExt};
    use message::{RoomJoinInfo, RtcConfig};
    use thiserror::Error;
    use tokio::sync::RwLock;
    use tracing::warn;
    use unicase::UniCase;
    use util::generate_random_string;

    use super::*;
    use std::{collections::HashMap, env::VarError, sync::Arc, time::SystemTimeError};

    #[derive(Clone, Default)]
    pub struct RoomProvider {
        rooms: Arc<RwLock<HashMap<UniCase<String>, Room>>>,
    }

    #[derive(Error, Debug)]
    pub enum RoomProviderError {
        #[error("cannot generate new key")]
        KeyGenerationFailed,
        #[error("given room does not exist")]
        RoomDoesntExist,

        #[error("RTCConfig Generation Failed")]
        RTCConfigGenerationFailed(#[from] VarError),

        #[error("SystemTime Error")]
        TimeError(#[from] SystemTimeError),

        #[error("Hmac InvalidLength error")]
        HmacError(#[from] sha1::digest::InvalidLength),
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
                    let id = UniCase::from(generate_random_string(6));
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
            let room = Room::new(user);
            let player_status = room.player_status.clone();
            rooms.insert(id.clone(), room);
            let rtc_config = get_rtc_info(&user_meta.name.to_string()).await?;
            Ok(RoomJoinInfo {
                room_id: id.to_lowercase(),
                user_id: user_meta.id,
                users: vec![user_meta],
                player_status,
                rtc_config,
            })
        }

        pub async fn join_room(
            &self,
            room_id: &str,
            user: User,
        ) -> Result<RoomJoinInfo, RoomProviderError> {
            let mut rooms = self.rooms.write().await;
            let user_id = user.meta.id;
            if let Some(room) = rooms.get_mut(&UniCase::from(room_id)) {
                room.users.push(user);
                let rtc_config = get_rtc_info(&user_id.to_string()).await?;
                Ok(RoomJoinInfo {
                    room_id: room_id.to_string(),
                    user_id,
                    users: room.users.iter().map(|u| u.meta.clone()).collect(),
                    player_status: room.player_status.clone(),
                    rtc_config,
                })
            } else {
                Err(RoomProviderError::RoomDoesntExist)
            }
        }

        pub async fn broadcast_msg_excluding(
            &self,
            room_id: &str,
            message: Message,
            excluded_users: &[Uuid],
        ) {
            let rooms = self.rooms.read().await;
            if let Some(room) = rooms.get(&UniCase::from(room_id)) {
                let send_futures = room
                    .users
                    .iter()
                    .filter(|user| !excluded_users.contains(&user.meta.id))
                    .map(|user| user.sender.send(message.clone()))
                    .collect::<FuturesUnordered<_>>();

                send_futures
                    .into_stream()
                    .for_each_concurrent(None, |data| async {
                        if let Err(err) = data {
                            warn!("broadcast failed {err:?}");
                        }
                    })
                    .await;
            }
        }

        pub async fn remove_user(&self, room_id: &str, user_id: Uuid) -> Option<Vec<UserMeta>> {
            let mut rooms = self.rooms.write().await;
            if let Some(room) = rooms.get_mut(&UniCase::from(room_id)) {
                room.users.retain(|user| user.meta.id != user_id);
                let users = room.users.iter().map(|u| u.meta.clone()).collect();
                if room.users.is_empty() {
                    rooms.remove(&UniCase::from(room_id));
                }
                Some(users)
            } else {
                None
            }
        }

        pub async fn get_room_player_status(&self, room_id: &str) -> Option<PlayerStatus> {
            let rooms = self.rooms.read().await;
            rooms
                .get(&UniCase::from(room_id))
                .map(|room| room.player_status.clone())
        }

        pub async fn with_room_mut<U>(
            &self,
            room_id: &str,
            f: impl FnOnce(&mut Room) -> U,
        ) -> Option<U> {
            let mut rooms = self.rooms.write().await;
            rooms.get_mut(&UniCase::from(room_id)).map(f)
        }

        pub async fn with_room<U>(&self, room_id: &str, f: impl FnOnce(&Room) -> U) -> Option<U> {
            let rooms = self.rooms.read().await;
            rooms.get(&UniCase::from(room_id)).map(f)
        }
    }

    impl Room {
        pub fn new(user: User) -> Self {
            Self {
                users: vec![user],
                player_status: PlayerStatus::Paused(0.0),
            }
        }
    }

    pub async fn get_rtc_info(username: &str) -> Result<message::RtcConfig, RoomProviderError> {
        use base64::prelude::*;
        use hmac::{Hmac, Mac};
        use sha1::Sha1;
        use std::time::SystemTime;
        use std::time::UNIX_EPOCH;

        const TTL: u64 = 3600;

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let timestamp = now + TTL;
        let turn_username = format!("{}:{}", timestamp, username);

        // Your TURN server's static auth secret
        let secret = std::env::var("TURN_SECRET")?;

        // Create the HMAC using secret and username
        let mut mac = Hmac::<Sha1>::new_from_slice(secret.as_bytes())?;
        mac.update(turn_username.as_bytes());
        let result = mac.finalize().into_bytes();

        // Base64 encode the resulting HMAC digest
        let credential = BASE64_STANDARD.encode(result);

        Ok(RtcConfig {
            stun: "stun:coturn.deepgaurav.com:3478".to_string(),
            turn: "turn:coturn.deepgaurav.com:3478?transport=udp".to_string(),
            turn_user: turn_username,
            turn_creds: credential,
        })
    }
}
