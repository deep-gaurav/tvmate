use axum::{
    extract::{ws::WebSocket, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use common::{
    message::{Message, RoomJoinInfo},
    message_sender::MessageSender,
    params::{HostParams, JoinParams},
    RoomProviderError, User, UserMeta,
};
use leptos::logging::warn;
use serde::{Deserialize, Serialize};
use thiserror::Error;
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
        },
        sender: tx,
    };
    let room_id = app_state.rooms.new_room(user).await?;

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
        },
        sender: tx,
    };
    let join_info = app_state
        .rooms
        .join_room(&join_params.room_id, user)
        .await?;
    let room_id = join_params.room_id;
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
                                            Ok(_) => {

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
                        break;
                    }
                }
            }
        }
    }
    app_state.rooms.remove_user(&room_id, user_id).await;
}

impl IntoResponse for RoomJoinError {
    fn into_response(self) -> Response {
        match self {
            RoomJoinError::RoomProviderError(err) => match err {
                RoomProviderError::KeyGenerationFailed => {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#?}")).into_response()
                }
                RoomProviderError::RoomDoesntExist => {
                    (StatusCode::BAD_REQUEST, format!("{err:#?}")).into_response()
                }
            },
        }
    }
}
