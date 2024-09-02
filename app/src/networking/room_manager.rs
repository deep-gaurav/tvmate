use std::{borrow::BorrowMut, cell::RefCell, marker::PhantomData, rc::Rc};

use codee::binary::BincodeSerdeCodec;
use common::{
    endpoints,
    message::Message,
    params::{HostParams, JoinParams},
    UserMeta,
};
use leptos::{
    create_effect, logging::warn, on_cleanup, store_value, Signal, SignalGet, StoredValue,
};
use leptos_router::use_navigate;
use leptos_use::{use_websocket, UseWebSocketReturn};
use thiserror::Error;
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
pub struct RoomManager {
    state: StoredValue<RoomState<Message>>,
}

pub enum RoomState<Tx>
where
    Tx: 'static,
{
    Disconnected,
    Connecting(WebsocketContext<Tx>),
    Connected(RoomConnectionInfo<Tx>),
}

pub struct RoomConnectionInfo<Tx>
where
    Tx: 'static,
{
    pub id: String,
    pub user_id: Uuid,
    pub users: Vec<UserMeta>,
    connection: WebsocketContext<Tx>,
}

#[derive(Error, Debug)]
pub enum RoomManagerError {
    #[error("already connected to room")]
    AlreadyConnectedToRoom,
    #[error("not connected to room")]
    NotConnectedToRoom,

    #[error("Param failed to encode")]
    ParamError(#[from] serde_urlencoded::ser::Error),
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            state: store_value(RoomState::Disconnected),
        }
    }

    pub fn host_join(
        &self,
        name: String,
        room_code: Option<String>,
    ) -> Result<Signal<Option<Message>>, RoomManagerError> {
        let is_connected = self
            .state
            .with_value(|v| matches!(v, RoomState::Connecting(_)));
        if is_connected {
            return Err(RoomManagerError::AlreadyConnectedToRoom);
        }
        let url = if room_code.is_some() {
            endpoints::JOIN_ROOM
        } else {
            endpoints::HOST_ROOM
        };
        let params = {
            if let Some(room_id) = room_code {
                let join_params = JoinParams { name, room_id };
                serde_urlencoded::to_string(&join_params)
            } else {
                let host_params = HostParams { name };
                serde_urlencoded::to_string(&host_params)
            }
        };
        match params {
            Ok(params) => {
                let UseWebSocketReturn {
                    send,
                    message,
                    close,
                    ..
                } = use_websocket::<Message, Message, BincodeSerdeCodec>(&format!(
                    "{url}?{params}"
                ));
                let messages_c = message.clone();
                let state_c = self.state.clone();
                create_effect(move |_| {
                    let message = messages_c.get();
                    info!("Received message {message:#?}");
                    if let Some(message) = message {
                        match message {
                            Message::ServerMessage(message) => match message {
                                common::message::ServerMessage::RoomCreated(room_info)
                                | common::message::ServerMessage::RoomJoined(room_info) => {
                                    let nav = use_navigate();
                                    state_c.update_value(|v| {
                                        if let RoomState::Connecting(connection) = v {
                                            *v = RoomState::Connected(RoomConnectionInfo {
                                                id: room_info.room_id.clone(),
                                                user_id: room_info.user_id.clone(),
                                                users: vec![],
                                                connection: unsafe { std::ptr::read(connection) },
                                            })
                                        }
                                    });
                                    nav(
                                        &format!("/room/{}", room_info.room_id),
                                        Default::default(),
                                    );
                                }
                            },
                            Message::ClientMessage => {}
                        }
                    } else {
                        close();
                        state_c.update_value(|v| *v = RoomState::Disconnected);
                    }
                });
                // info!("is connecting {is_connecting}");
                self.state.update_value(|val| {
                    *val = RoomState::Connecting(WebsocketContext::new(
                        message.clone(),
                        Box::new(send),
                    ));
                });
                Ok(message)
            }
            Err(err) => {
                warn!("Cant serialize params {err:#?}");
                Err(err.into())
            }
        }
    }

    pub fn message_signal(&self) -> Result<Signal<Option<Message>>, RoomManagerError> {
        self.state.with_value(|val| match val {
            RoomState::Disconnected => Err(RoomManagerError::NotConnectedToRoom),
            RoomState::Connecting(connection) => Ok(connection.message.clone()),

            RoomState::Connected(room_info) => Ok(room_info.connection.message.clone()),
        })
    }
}

pub struct WebsocketContext<Tx>
where
    Tx: 'static,
{
    pub message: Signal<Option<Message>>,
    send: Box<dyn Fn(&Tx)>, // use Rc to make it easily cloneable
    _phantom: PhantomData<Tx>,
}

impl<Tx> WebsocketContext<Tx>
where
    Tx: 'static,
{
    pub fn new(message: Signal<Option<Message>>, send: Box<dyn Fn(&Tx)>) -> Self {
        Self {
            message,
            send,
            _phantom: PhantomData,
        }
    }

    // create a method to avoid having to use parantheses around the field
    #[inline(always)]
    pub fn send(&self, message: &Tx) {
        (self.send)(&message)
    }
}
