use std::{
    cell::{Ref, RefCell},
    marker::PhantomData,
    rc::Rc,
};

use codee::binary::BincodeSerdeCodec;
use common::{
    endpoints,
    message::{Message, UserJoined, UserLeft},
    params::{HostParams, JoinParams},
    UserMeta,
};
use leptos::{
    create_effect, create_signal, logging::warn, store_value, ReadSignal, Signal, SignalGet,
    SignalGetUntracked, SignalSet, StoredValue, WriteSignal,
};
use leptos_router::use_navigate;
use leptos_use::{
    use_websocket, use_websocket_with_options, UseWebSocketOptions, UseWebSocketReturn,
};
use thiserror::Error;
use tracing::{debug, info};
use uuid::Uuid;

use crate::components::room_info;

#[derive(Clone)]
pub struct RoomManager {
    state: Rc<RefCell<RoomState<Message>>>,
    room_info_signal: (ReadSignal<Option<RoomInfo>>, WriteSignal<Option<RoomInfo>>),
}

pub enum RoomState<Tx>
where
    Tx: 'static,
{
    Disconnected,
    Connecting(WebsocketContext<Tx>),
    Connected(RoomConnectionInfo<Tx>),
}

impl<Tx> RoomState<Tx>
where
    Tx: 'static,
{
    /// Returns `true` if the room state is [`Connecting`].
    ///
    /// [`Connecting`]: RoomState::Connecting
    #[must_use]
    pub fn is_connecting(&self) -> bool {
        matches!(self, Self::Connecting(..))
    }

    /// Returns `true` if the room state is [`Connected`].
    ///
    /// [`Connected`]: RoomState::Connected
    #[must_use]
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected(..))
    }

    /// Returns `true` if the room state is [`Disconnected`].
    ///
    /// [`Disconnected`]: RoomState::Disconnected
    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        matches!(self, Self::Disconnected)
    }

    pub fn as_connected(&self) -> Option<&RoomConnectionInfo<Tx>> {
        if let Self::Connected(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

pub struct RoomConnectionInfo<Tx>
where
    Tx: 'static,
{
    pub connection: WebsocketContext<Tx>,
}

#[derive(Debug, Clone)]
pub struct RoomInfo {
    pub id: String,
    pub user_id: Uuid,
    pub users: Vec<UserMeta>,
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
            state: Rc::new(RefCell::new(RoomState::Disconnected)),
            room_info_signal: create_signal(None),
        }
    }

    pub fn get_room_info(&self) -> ReadSignal<Option<RoomInfo>> {
        self.room_info_signal.0
    }

    pub fn host_join(
        &self,
        name: String,
        room_code: Option<String>,
    ) -> Result<Signal<Option<Message>>, RoomManagerError> {
        let is_disconnected = self.state.borrow().is_disconnected();
        if !is_disconnected {
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
                    ready_state,
                    close,
                    ..
                } = use_websocket_with_options::<Message, Message, BincodeSerdeCodec>(
                    &format!("{url}?{params}"),
                    UseWebSocketOptions::default()
                        .reconnect_limit(leptos_use::ReconnectLimit::Limited(0)),
                );
                let messages_c = message.clone();
                let state_c = self.state.clone();
                let state_c1 = self.state.clone();
                let room_info_reader = self.room_info_signal.0;
                let room_info_writer = self.room_info_signal.1;
                create_effect(move |_| {
                    let ws_state = ready_state.get();
                    info!("WS State change {:#?}", ws_state);
                    match ws_state {
                        leptos_use::core::ConnectionReadyState::Connecting => {
                            info!("Connecting to ws")
                        }
                        leptos_use::core::ConnectionReadyState::Open => {
                            info!("Opened ws")
                        }
                        leptos_use::core::ConnectionReadyState::Closing
                        | leptos_use::core::ConnectionReadyState::Closed => {
                            // close();
                            info!("Borrow mut for disconnect");
                            let mut state = state_c1.borrow_mut();
                            *state = RoomState::Disconnected;
                            drop(state);
                            room_info_writer.set(None);
                        }
                    }
                });
                create_effect(move |_| {
                    let message = messages_c.get();
                    info!("Received message {message:#?}");
                    if let Some(message) = message {
                        match message {
                            Message::ServerMessage(message) => match message {
                                common::message::ServerMessage::RoomCreated(room_info)
                                | common::message::ServerMessage::RoomJoined(room_info) => {
                                    let nav = use_navigate();
                                    let state_c_ref = state_c.borrow();
                                    if let RoomState::Connecting(connection) = &*state_c_ref {
                                        let room_info = RoomInfo {
                                            id: room_info.room_id.clone(),
                                            user_id: room_info.user_id.clone(),
                                            users: room_info.users,
                                        };
                                        let connection_info = RoomConnectionInfo {
                                            connection: unsafe { std::ptr::read(connection) },
                                        };
                                        drop(state_c_ref);
                                        info!("Borrow mut for connected");
                                        let mut state = state_c.borrow_mut();
                                        *state = RoomState::Connected(connection_info);
                                        drop(state);
                                        nav(&format!("/room/{}", room_info.id), Default::default());
                                        room_info_writer.set(Some(room_info));
                                    }
                                }
                                common::message::ServerMessage::UserJoined(UserJoined {
                                    new_user,
                                    users,
                                }) => {
                                    let room_info = room_info_reader.get_untracked();
                                    if let Some(mut room_info) = room_info {
                                        room_info.users = users;
                                        room_info_writer.set(Some(room_info));
                                    }
                                }
                                common::message::ServerMessage::UserLeft(UserLeft {
                                    user_left,
                                    users,
                                }) => {
                                    let room_info = room_info_reader.get_untracked();
                                    if let Some(mut room_info) = room_info {
                                        room_info.users = users;
                                        room_info_writer.set(Some(room_info));
                                    }
                                }
                            },
                            Message::ClientMessage => {}
                        }
                    } else {
                        info!("Received nothing, closing");
                        // close();
                    }
                });
                // info!("is connecting {is_connecting}");
                info!("Borrow mut for connecting");
                let mut state = self.state.borrow_mut();
                *state =
                    RoomState::Connecting(WebsocketContext::new(message.clone(), Box::new(send)));
                drop(state);
                Ok(message)
            }
            Err(err) => {
                warn!("Cant serialize params {err:#?}");
                Err(err.into())
            }
        }
    }

    pub fn message_signal(&self) -> Result<Signal<Option<Message>>, RoomManagerError> {
        let val = self.state.borrow();
        match &*val {
            RoomState::Disconnected => Err(RoomManagerError::NotConnectedToRoom),
            RoomState::Connecting(connection) => Ok(connection.message.clone()),

            RoomState::Connected(room_info) => Ok(room_info.connection.message.clone()),
        }
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
