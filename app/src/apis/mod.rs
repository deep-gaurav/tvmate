use leptos::{expect_context, server, use_context, ServerFnError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct RoomMetaInfo {
    pub room_id: String,
    pub host: String,
    pub selected_video: Option<String>,
}

#[server]
pub async fn get_room_info(room_id: String) -> Result<Option<RoomMetaInfo>, ServerFnError> {
    use common::RoomProvider;

    let rooms = use_context::<RoomProvider>().ok_or(ServerFnError::new("RoomProvider expected"))?;

    Ok(rooms
        .with_room(&room_id, |room| {
            room.users.first().map(|host| RoomMetaInfo {
                room_id: room_id.clone(),
                host: host.meta.name.clone(),
                selected_video: host.meta.state.as_video_selected().cloned(),
            })
        })
        .await
        .flatten())
}
