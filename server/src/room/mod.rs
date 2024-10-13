use axum::{
    extract::{
        ws::{self, CloseFrame, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use common::{
    message::{ClientMessage, Message, UserJoined, UserLeft},
    message_sender::MessageSender,
    params::{HostParams, JoinParams},
    PlayerStatus, RoomProviderError, User, UserMeta, UserState,
};
use leptos::logging::warn;
use thiserror::Error;
use tracing::info;
use uuid::Uuid;

use crate::AppState;

#[derive(Error, Debug)]
pub enum RoomJoinError {
    #[error(transparent)]
    RoomProviderError(#[from] RoomProviderError),
}

#[axum::debug_handler]
pub async fn host_room(
    State(app_state): State<AppState>,
    Query(host_params): Query<HostParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, RoomJoinError> {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let user_id = Uuid::new_v4();
    let user = User {
        meta: UserMeta {
            id: user_id,
            name: host_params.name,
            state: common::UserState::VideoNotSelected,
        },
        sender: tx,
        last_chat_request: None,
    };
    let room_id = app_state.rooms.new_room(user).await;

    let room_id = match room_id {
        Ok(r) => r,
        Err(er) => {
            warn!("Failed to create room {er:?}");
            return Err(er.into());
        }
    };
    Ok(ws.on_upgrade(move |mut msgs| async move {
        msgs.send_message(&Message::ServerMessage(
            common::message::ServerMessage::RoomCreated(room_id.clone()),
        ))
        .await;

        handle_websocket(app_state, &room_id.room_id, user_id, msgs, rx).await;
    }))
}

#[axum::debug_handler]
pub async fn join_room(
    State(app_state): State<AppState>,
    Query(join_params): Query<JoinParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, RoomJoinError> {
    let (tx, rx) = tokio::sync::mpsc::channel(10); // 10 is random here.
    let user_id = Uuid::new_v4();
    let user = User {
        meta: UserMeta {
            id: user_id,
            name: join_params.name,

            state: common::UserState::VideoNotSelected,
        },
        sender: tx,
        last_chat_request: None,
    };

    let join_info = match app_state
        .rooms
        .join_room(&join_params.room_id.to_lowercase(), user)
        .await
    {
        Ok(info) => info,
        Err(error) => {
            return Ok(ws.on_upgrade(move |mut sock| async move {
                if let Err(err) = sock
                    .send(axum::extract::ws::Message::Close(Some(CloseFrame {
                        code: ws::close_code::POLICY,
                        reason: error.to_string().into(),
                    })))
                    .await
                {
                    warn!("Cant send close {err:?}");
                }
            }))
        }
    };
    let room_id = join_params.room_id;
    if let Some(player_status) = app_state.rooms.get_room_player_status(&room_id).await {
        app_state
            .rooms
            .broadcast_msg_excluding(
                &room_id,
                Message::ServerMessage(common::message::ServerMessage::UserJoined(UserJoined {
                    new_user: join_info.user_id,
                    users: join_info.users.clone(),
                    player_status,
                })),
                &[join_info.user_id],
            )
            .await;
    }
    Ok(ws.on_upgrade(move |mut msgs| async move {
        msgs.send_message(&Message::ServerMessage(
            common::message::ServerMessage::RoomJoined(join_info),
        ))
        .await;

        handle_websocket(app_state, &room_id, user_id, msgs, rx).await;
    }))
}

async fn handle_websocket(
    app_state: AppState,
    room_id: &str,
    user_id: Uuid,
    mut socket: WebSocket,
    mut rx: tokio::sync::mpsc::Receiver<Message>,
) {
    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(msg) => {
                        match msg {
                            Ok(msg) => {
                                match msg {
                                    axum::extract::ws::Message::Text(_) => {
                                        //ignore
                                    },
                                    axum::extract::ws::Message::Binary(data) => {
                                        let data = bincode::deserialize::<Message>(&data[..]);
                                        match data {
                                            Ok(original_message) => {
                                                match &original_message {
                                                    Message::ServerMessage(_) => {
                                                        //ignore
                                                    },
                                                    Message::ClientMessage((sender_id, message)) => {
                                                        if sender_id == &user_id {
                                                            match message {
                                                                common::message::ClientMessage::Chat(_) => {
                                                                    app_state.rooms.broadcast_msg_excluding(room_id, original_message, &[user_id]).await;
                                                                }
                                                                common::message::ClientMessage::SelectedVideo(video_name) => {
                                                                    app_state.rooms.with_room_mut(room_id, |room|{
                                                                        if let Some(user) = room.users.iter_mut().find(|u|u.meta.id == user_id)
                                                                        {
                                                                            user.meta.state = UserState::VideoSelected(video_name.clone());
                                                                        }
                                                                    }).await;
                                                                    app_state.rooms.broadcast_msg_excluding(room_id, original_message, &[user_id]).await;
                                                                },
                                                                common::message::ClientMessage::Play(val) => {
                                                                    app_state.rooms.with_room_mut(room_id, |room|{
                                                                        room.player_status = PlayerStatus::Playing(*val);
                                                                    }).await;
                                                                    app_state.rooms.broadcast_msg_excluding(room_id, original_message, &[user_id]).await;
                                                                },
                                                                common::message::ClientMessage::Pause(val) => {
                                                                    app_state.rooms.with_room_mut(room_id, |room|{
                                                                        room.player_status = PlayerStatus::Paused(*val);
                                                                    }).await;
                                                                    app_state.rooms.broadcast_msg_excluding(room_id, original_message, &[user_id]).await;
                                                                },
                                                                common::message::ClientMessage::Seek(val) | common::message::ClientMessage::Update(val) => {
                                                                    app_state.rooms.with_room_mut(room_id, |room|{
                                                                        match &mut room.player_status {
                                                                            PlayerStatus::Paused(time) | PlayerStatus::Playing(time) => *time = *val,
                                                                        }
                                                                    }).await;
                                                                    app_state.rooms.broadcast_msg_excluding(room_id, original_message, &[user_id]).await;
                                                                },
                                                                common::message::ClientMessage::SendSessionDesc(uuid, rtcsession_desc) => {
                                                                    info!("Sending description from {sender_id} to {uuid}");
                                                                    let sender = app_state.rooms.with_room(room_id, |room| {
                                                                        room.users.iter().find(|user|user.meta.id == *uuid).map(|user| user.sender.clone())
                                                                    }).await.flatten();
                                                                    if let Some(sender) = sender {
                                                                        if let Err(err) = sender.send(Message::ClientMessage((*sender_id, ClientMessage::ReceivedSessionDesc(rtcsession_desc.clone())))).await{
                                                                            warn!("Failed send session desc {err:?}");
                                                                        }
                                                                        info!("sent description from {sender_id} to {uuid}");

                                                                    }else{
                                                                        warn!("User {uuid} not found");
                                                                    }
                                                                },

                                                                common::message::ClientMessage::ExchangeCandidate(uuid, candidate) => {
                                                                    let sender = app_state.rooms.with_room(room_id, |room| {
                                                                        room.users.iter().find(|user|user.meta.id == *uuid).map(|user| user.sender.clone())
                                                                    }).await.flatten();
                                                                    if let Some(sender) = sender {
                                                                        if let Err(err) = sender.send(Message::ClientMessage((*sender_id, ClientMessage::ExchangeCandidate(*sender_id,candidate.clone())))).await{
                                                                            warn!("Failed send session desc {err:?}");
                                                                        }
                                                                    }
                                                                },
                                                                common::message::ClientMessage::RequestCall(uuid, video,audio) => 'b:{
                                                                    if let Some((Some(last_send), sender)) = app_state.rooms.with_room(room_id,|room|{
                                                                        room.users.iter().find(|user|user.meta.id == *sender_id).map(|u|(u.last_chat_request, u.sender.clone()))
                                                                    }).await.flatten() {
                                                                        if std::time::Instant::now().duration_since(last_send) < std::time::Duration::from_secs(60) {
                                                                            if let Err(err) = sender.send(Message::ServerMessage(common::message::ServerMessage::Error("Cant send vc request, Try after some time".to_string()))).await{
                                                                                warn!("Failed to send error {err:?}");
                                                                            }
                                                                            info!("Frequent request, ignoring");
                                                                            break 'b;
                                                                        }

                                                                    }
                                                                    let sender = app_state.rooms.with_room_mut(room_id, |room| {
                                                                        room.users.iter_mut().find(|user|user.meta.id == *uuid).map(|user| {
                                                                            user.last_chat_request = Some(std::time::Instant::now()) ;
                                                                            user.sender.clone()
                                                                        })
                                                                    }).await.flatten();
                                                                    if let Some(sender) = sender {
                                                                        if let Err(err) = sender.send(Message::ClientMessage((*sender_id, ClientMessage::RequestCall(*sender_id, *video, *audio)))).await{
                                                                            warn!("Failed send vc request {err:?}");
                                                                        }
                                                                    }else{
                                                                        warn!("User doesnt exist, cant send vc request")
                                                                    }
                                                                },
                                                                common::message::ClientMessage::ReceivedSessionDesc(_rtcsession_desc) => {
                                                                    warn!("Shouldnt receive received desc");
                                                                },
                                                            }
                                                        }
                                                    },
                                                }
                                            },
                                            Err(err) => {
                                                warn!("Received msg decode error {err:#?}")
                                            },
                                        }
                                    },
                                    axum::extract::ws::Message::Ping(_) => {
                                        //ignore
                                    },
                                    axum::extract::ws::Message::Pong(_) => {
                                        //ignore
                                    },
                                    axum::extract::ws::Message::Close(_) => {
                                        info!("Received Close from socket disconnecting {user_id}");
                                        break;
                                    },
                                }
                            }
                            Err(err) => {
                                warn!("Msg receive error {err:#?}")
                            }
                        }
                    },
                    None => {
                        // User disconnected
                        info!("Received None from socket disconnecting {user_id}");
                        break;
                    },
                }
            }
            msg = rx.recv() => {
                match msg {
                    Some(msg) => {
                        socket.send_message(&msg).await;
                    }
                    None => {
                        // Sender dropped, room closed?
                        info!("Received None from rx disconnecting {user_id}");
                        break;
                    }
                }
            }
        }
    }
    let remaining_users = app_state.rooms.remove_user(room_id, user_id).await;
    info!("Disconnected user {user_id}");
    if let Some(users) = remaining_users {
        if let Some(player_status) = app_state.rooms.get_room_player_status(room_id).await {
            app_state
                .rooms
                .broadcast_msg_excluding(
                    room_id,
                    Message::ServerMessage(common::message::ServerMessage::UserLeft(UserLeft {
                        user_left: user_id,
                        users,
                        player_status,
                    })),
                    &[user_id],
                )
                .await;
        }
    }
}

impl IntoResponse for RoomJoinError {
    fn into_response(self) -> Response {
        match self {
            RoomJoinError::RoomProviderError(err) => match err {
                RoomProviderError::KeyGenerationFailed
                | RoomProviderError::RTCConfigGenerationFailed(_)
                | RoomProviderError::TimeError(_)
                | RoomProviderError::HmacError(_) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#?}")).into_response()
                }
                RoomProviderError::RoomDoesntExist | RoomProviderError::RoomFull => {
                    (StatusCode::BAD_REQUEST, format!("{err:#?}")).into_response()
                }
            },
        }
    }
}
