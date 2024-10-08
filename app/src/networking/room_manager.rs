use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use codee::binary::BincodeSerdeCodec;
use common::{
    endpoints,
    message::{ClientMessage, Message, RTCSessionDesc, RtcConfig, UserJoined, UserLeft},
    params::{HostParams, JoinParams},
    PlayerStatus, UserMeta, UserState,
};
use leptos::{
    create_effect, create_signal, expect_context, logging::warn, store_value, with_owner, Callback,
    Owner, ReadSignal, Signal, SignalGet, SignalGetUntracked, SignalSet, SignalWith,
    SignalWithUntracked, StoredValue, WriteSignal,
};
use leptos_router::use_navigate;
use leptos_use::{
    core::ConnectionReadyState, use_event_listener, use_websocket_with_options,
    UseWebSocketOptions, UseWebSocketReturn,
};
use thiserror::Error;
use tracing::info;
use uuid::Uuid;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    js_sys::Array, MediaStream, MediaStreamTrack, RtcIceCandidateInit, RtcPeerConnection,
    RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescriptionInit, RtcTrackEvent, WebSocket,
};

use crate::{
    components::toaster::{Toast, Toaster},
    networking::rtc_connect::{
        add_media_tracks, connect_rtc, deserialize_candidate, serialize_candidate,
    },
    Endpoint,
};

use super::rtc_connect::{connect_to_user, get_media_stream, receive_peer_connections};

#[derive(Clone)]
pub struct RoomManager {
    state: Rc<RefCell<RoomState<Message>>>,
    room_info_signal: (ReadSignal<Option<RoomInfo>>, WriteSignal<Option<RoomInfo>>),
    player_message_tx: (
        ReadSignal<Option<PlayerMessages>>,
        WriteSignal<Option<PlayerMessages>>,
    ),
    pub audio_chat_stream_signal: (
        ReadSignal<Option<(Uuid, MediaStream)>>,
        WriteSignal<Option<(Uuid, MediaStream)>>,
    ),
    pub video_chat_stream_signal: (
        ReadSignal<Option<(Uuid, MediaStream)>>,
        WriteSignal<Option<(Uuid, MediaStream)>>,
    ),
    pub rtc_signal: (
        ReadSignal<Option<(Uuid, RtcPeerConnection)>>,
        WriteSignal<Option<(Uuid, RtcPeerConnection)>>,
    ),
    pub ice_signal: (
        ReadSignal<Option<(Uuid, String)>>,
        WriteSignal<Option<(Uuid, String)>>,
    ),
    pub sdp_signal: (
        ReadSignal<Option<(Uuid, RTCSessionDesc)>>,
        WriteSignal<Option<(Uuid, RTCSessionDesc)>>,
    ),
    pub vc_permission: StoredValue<HashMap<Uuid, (bool, bool)>>,

    pub permission_request_signal: Signal<Option<(Uuid, bool, bool)>>,
    permission_request_sender: WriteSignal<Option<(Uuid, bool, bool)>>,
    owner: Owner,
}

pub enum RoomState<Tx>
where
    Tx: 'static,
{
    Disconnected,
    Connecting(
        (
            WebsocketContext<Tx>,
            Signal<Option<WebSocket>>,
            Signal<ConnectionReadyState>,
        ),
    ),
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
    pub socket: Signal<Option<WebSocket>>,
    pub ready_state: Signal<ConnectionReadyState>,
    pub chat_history: StoredValue<Vec<(UserMeta, String)>>,
    pub chat_signal: (
        ReadSignal<Option<(UserMeta, String)>>,
        WriteSignal<Option<(UserMeta, String)>>,
    ),
    pub rtc_config: StoredValue<RtcConfig>,
    pub rtc_peers: StoredValue<HashMap<Uuid, RtcPeerConnection>>,
    pub rtc_pending_ice: StoredValue<HashMap<Uuid, Vec<RtcIceCandidateInit>>>,
}

#[derive(Debug, Clone)]
pub struct RoomInfo {
    pub id: String,
    pub user_id: Uuid,
    pub users: Vec<UserMeta>,
    pub player_status: PlayerStatus,
}

#[derive(Clone)]
pub enum PlayerMessages {
    Play(f64),
    Pause(f64),
    Update(f64),
    Seek(f64),
}

pub enum SendType {
    Reliable,
    UnReliablle,
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
    pub fn new(owner: Owner) -> Self {
        let state = Rc::new(RefCell::new(RoomState::Disconnected));
        let room_info_signal = with_owner(owner, || create_signal(None));
        let (ice_read, ice_tx) = with_owner(owner, || create_signal(None));
        let (session_description, session_description_tx) = create_signal(None);
        let (video_rx, video_tx) = with_owner(owner, || create_signal(None));
        let (audio_rx, audio_tx) = with_owner(owner, || create_signal(None));
        let (rtc_rx, rtc_tx) = with_owner(owner, || create_signal(None));
        let vc_permission = store_value(HashMap::new());

        let (permissions_rx, permissions_tx) = create_signal(None);

        let rm = Self {
            state,
            room_info_signal,
            player_message_tx: create_signal(None),
            audio_chat_stream_signal: (audio_rx, audio_tx),
            video_chat_stream_signal: (video_rx, video_tx),
            ice_signal: (ice_read, ice_tx),
            sdp_signal: (session_description, session_description_tx),
            owner,
            vc_permission,
            permission_request_sender: permissions_tx,
            permission_request_signal: permissions_rx.into(),
            rtc_signal: (rtc_rx, rtc_tx),
        };
        with_owner(owner, {
            let rm = rm.clone();
            let state = rm.state.clone();
            move || {
                receive_peer_connections(
                    Callback::new(move |_| {
                        room_info_signal
                            .0
                            .with(|room| room.as_ref().map(|r: &RoomInfo| r.user_id))
                    }),
                    {
                        let state = state.clone();
                        Callback::new(move |_| {
                            let rtc_config_peer =
                                if let RoomState::Connected(RoomConnectionInfo {
                                    rtc_config, ..
                                }) = &*state.borrow()
                                {
                                    Some(*rtc_config)
                                } else {
                                    None
                                };
                            rtc_config_peer.map(|s| s.get_value())
                        })
                    },
                    Callback::new(move |user_id| {
                        vc_permission
                            .with_value(|p| p.get(&user_id).cloned())
                            .unwrap_or((false, false))
                    }),
                    Callback::new(move |(user, stream)| {
                        video_tx.set(Some((user, stream)));
                    }),
                    Callback::new(move |(user, stream)| {
                        audio_tx.set(Some((user, stream)));
                    }),
                    Callback::new(move |(user, pc)| {
                        rtc_tx.set(Some((user, pc)));
                    }),
                    {
                        let rm = rm.clone();
                        Callback::new(move |(user, ice)| {
                            info!("Send ice {user} {ice:?}");
                            rm.send_message(
                                ClientMessage::ExchangeCandidate(user, ice),
                                SendType::Reliable,
                            );
                        })
                    },
                    {
                        let rm = rm.clone();
                        Callback::new(move |(user, sdp)| {
                            info!("Send sdp {user} {sdp:?}");
                            rm.send_message(
                                ClientMessage::SendSessionDesc(user, sdp),
                                SendType::Reliable,
                            );
                        })
                    },
                    ice_read.into(),
                    session_description.into(),
                    owner,
                );
            }
        });

        rm
    }

    pub fn get_room_info(&self) -> ReadSignal<Option<RoomInfo>> {
        self.room_info_signal.0
    }

    pub fn get_player_messages(&self) -> ReadSignal<Option<PlayerMessages>> {
        self.player_message_tx.0
    }

    pub fn host_join(
        &self,
        name: String,
        room_code: Option<String>,
    ) -> Result<Signal<Option<Message>>, RoomManagerError> {
        let toaster = expect_context::<Toaster>();
        toaster.toast(Toast {
            message: "Connecting to server".into(),
            r#type: crate::components::toaster::ToastType::Info,
        });
        with_owner(self.owner, || {
            let owner = self.owner;
            let is_disconnected = self.state.borrow().is_disconnected();
            if !is_disconnected {
                toaster.toast(Toast {
                    message: "Already connected to a room".into(),
                    r#type: crate::components::toaster::ToastType::Failed,
                });
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
            let main_endpoint = expect_context::<Endpoint>().main_endpoint;
            match params {
                Ok(params) => {
                    let UseWebSocketReturn {
                        send,
                        message,
                        ready_state,
                        ws,
                        ..
                    } = use_websocket_with_options::<Message, Message, BincodeSerdeCodec>(
                        &format!("{main_endpoint}{url}?{params}"),
                        UseWebSocketOptions::default()
                            .reconnect_limit(leptos_use::ReconnectLimit::Limited(0))
                            .on_error(move |err| {
                                toaster.toast(Toast {
                                    message: "Connection Failed".to_string().into(),
                                    r#type: crate::components::toaster::ToastType::Failed,
                                });
                            })
                            .on_close(move |ev| {
                                let reason = ev.reason();
                                toaster.toast(Toast {
                                    message: reason.into(),
                                    r#type: crate::components::toaster::ToastType::Failed,
                                });
                            }),
                    );
                    let state_c = self.state.clone();
                    let state_c1 = self.state.clone();
                    let room_info_reader = self.room_info_signal.0;
                    let room_info_writer = self.room_info_signal.1;
                    let player_messages_sender = self.player_message_tx.1;
                    create_effect(move |_| {
                        let ws_state = ready_state.get();
                        info!("WS State change {:#?}", ws_state);
                        match ws_state {
                            leptos_use::core::ConnectionReadyState::Connecting => {
                                info!("Connecting to ws")
                            }
                            leptos_use::core::ConnectionReadyState::Open => {
                                toaster.toast(Toast {
                                    message: "Connection Successful".into(),
                                    r#type: crate::components::toaster::ToastType::Success,
                                });
                                info!("Opened ws")
                            }
                            leptos_use::core::ConnectionReadyState::Closing
                            | leptos_use::core::ConnectionReadyState::Closed => {
                                // close();
                                let mut state = state_c1.borrow_mut();
                                *state = RoomState::Disconnected;
                                drop(state);
                                room_info_writer.set(None);
                            }
                        }
                    });
                    let rm = self.clone();

                    let ice_setter = self.ice_signal.1;
                    let sdp_setter = self.sdp_signal.1;

                    let permission_request_notifier = self.permission_request_sender;

                    create_effect(move |_| {
                        let message = message.get();
                        if let Some(message) = message {
                            match message {
                                Message::ServerMessage(message) => match message {
                                    common::message::ServerMessage::RoomCreated(room_info)
                                    | common::message::ServerMessage::RoomJoined(room_info) => {
                                        let nav = use_navigate();
                                        let state_c_ref = state_c.borrow();
                                        if let RoomState::Connecting((
                                            connection,
                                            socket,
                                            ready_state,
                                        )) = &*state_c_ref
                                        {
                                            let rtc_config = with_owner(owner, || {
                                                store_value(room_info.rtc_config)
                                            });
                                            let room_info = RoomInfo {
                                                id: room_info.room_id.clone(),
                                                user_id: room_info.user_id,
                                                users: room_info.users,
                                                player_status: room_info.player_status,
                                            };

                                            let chat_signal =
                                                with_owner(owner, || create_signal(None));
                                            let chat_history =
                                                with_owner(owner, || store_value(Vec::new()));

                                            with_owner(owner, || {
                                                create_effect(move |_| {
                                                    if let Some(msg) = chat_signal.0.get() {
                                                        chat_history.update_value(|v| v.push(msg));
                                                    }
                                                })
                                            });

                                            let pcs =
                                                with_owner(owner, || store_value(HashMap::new()));

                                            let ice =
                                                with_owner(owner, || store_value(HashMap::new()));

                                            let connection_info = RoomConnectionInfo {
                                                connection: unsafe { std::ptr::read(connection) },
                                                socket: *socket,
                                                ready_state: unsafe { std::ptr::read(ready_state) },
                                                chat_signal,
                                                chat_history,
                                                rtc_peers: pcs,
                                                rtc_config,
                                                rtc_pending_ice: ice,
                                            };
                                            drop(state_c_ref);
                                            let mut state = state_c.borrow_mut();
                                            *state = RoomState::Connected(connection_info);
                                            drop(state);
                                            nav(
                                                &format!("/room/{}", room_info.id),
                                                Default::default(),
                                            );
                                            room_info_writer.set(Some(room_info));
                                        }
                                    }
                                    common::message::ServerMessage::UserJoined(UserJoined {
                                        new_user,
                                        users,
                                        player_status,
                                    }) => {
                                        let room_info = room_info_reader.get_untracked();
                                        if let Some(mut room_info) = room_info {
                                            room_info.users = users;
                                            room_info.player_status = player_status;
                                            room_info_writer.set(Some(room_info));
                                        }
                                    }
                                    common::message::ServerMessage::UserLeft(UserLeft {
                                        user_left,
                                        users,
                                        player_status,
                                    }) => {
                                        let room_info = room_info_reader.get_untracked();
                                        if let Some(mut room_info) = room_info {
                                            room_info.users = users;
                                            room_info.player_status = player_status;
                                            room_info_writer.set(Some(room_info));
                                        }
                                    }
                                },
                                Message::ClientMessage((from_user, message)) => match message {
                                    common::message::ClientMessage::SelectedVideo(video_name) => {
                                        let room_info = room_info_reader.get_untracked();
                                        if let Some(mut room_info) = room_info {
                                            if let Some(user) = room_info
                                                .users
                                                .iter_mut()
                                                .find(|u| u.id == from_user)
                                            {
                                                user.state = UserState::VideoSelected(video_name);
                                                room_info_writer.set(Some(room_info));
                                            }
                                        }
                                    }
                                    common::message::ClientMessage::Play(time) => {
                                        if let Some(mut room_info) =
                                            room_info_reader.get_untracked()
                                        {
                                            room_info.player_status = PlayerStatus::Playing(time);
                                            room_info_writer.set(Some(room_info));
                                        }
                                        player_messages_sender
                                            .set(Some(PlayerMessages::Play(time)));
                                    }
                                    common::message::ClientMessage::Pause(time) => {
                                        if let Some(mut room_info) =
                                            room_info_reader.get_untracked()
                                        {
                                            room_info.player_status = PlayerStatus::Paused(time);
                                            room_info_writer.set(Some(room_info));
                                        }
                                        player_messages_sender
                                            .set(Some(PlayerMessages::Pause(time)));
                                    }
                                    common::message::ClientMessage::Seek(time) => {
                                        if let Some(mut room_info) =
                                            room_info_reader.get_untracked()
                                        {
                                            match &mut room_info.player_status {
                                                PlayerStatus::Paused(val)
                                                | PlayerStatus::Playing(val) => {
                                                    *val = time;
                                                }
                                            }
                                            room_info_writer.set(Some(room_info));
                                        }
                                        player_messages_sender
                                            .set(Some(PlayerMessages::Seek(time)));
                                    }
                                    common::message::ClientMessage::Update(time) => {
                                        if let Some(mut room_info) =
                                            room_info_reader.get_untracked()
                                        {
                                            match &mut room_info.player_status {
                                                PlayerStatus::Paused(val)
                                                | PlayerStatus::Playing(val) => {
                                                    *val = time;
                                                }
                                            }
                                            room_info_writer.set(Some(room_info));
                                        }
                                        player_messages_sender
                                            .set(Some(PlayerMessages::Update(time)));
                                    }
                                    common::message::ClientMessage::Chat(message) => {
                                        if let RoomState::Connected(RoomConnectionInfo {
                                            chat_signal,
                                            ..
                                        }) = &*state_c.borrow()
                                        {
                                            if let Some(user) = room_info_reader.with(|r| {
                                                r.as_ref().and_then(|r| {
                                                    r.users
                                                        .iter()
                                                        .find(|u| u.id == from_user)
                                                        .cloned()
                                                })
                                            }) {
                                                chat_signal.1.set(Some((user, message)));
                                            }
                                        }
                                    }
                                    ClientMessage::SendSessionDesc(_uuid, _rtcsession_desc) => {
                                        warn!("Received send session description")
                                    }
                                    ClientMessage::ExchangeCandidate(_uuid, ice) => {
                                        info!("Received ice from {from_user} {ice}");
                                        ice_setter.set(Some((from_user, ice)));
                                    }

                                    ClientMessage::ReceivedSessionDesc(sdp) => {
                                        info!("Received sdp from {from_user} {sdp:?}");
                                        sdp_setter.set(Some((from_user, sdp)));
                                    }
                                    ClientMessage::RequestCall(_, video, audio) => {
                                        info!("Receivedd vc request");
                                        permission_request_notifier
                                            .set(Some((from_user, video, audio)));
                                    }
                                },
                            }
                        } else {
                            info!("Received nothing, closing");
                            // close();
                        }
                    });
                    // info!("is connecting {is_connecting}");
                    info!("Borrow mut for connecting");
                    let (message_sender_rx, message_sender_tx) = create_signal(None);
                    create_effect(move |_| {
                        if let Some(message) = message_sender_rx.get() {
                            send(&message);
                        }
                    });
                    let mut state = self.state.borrow_mut();
                    *state = RoomState::Connecting((
                        WebsocketContext::new(message, message_sender_tx),
                        ws,
                        ready_state,
                    ));
                    drop(state);
                    Ok(message)
                }
                Err(err) => {
                    warn!("Cant serialize params {err:#?}");
                    Err(err.into())
                }
            }
        })
    }

    pub fn message_signal(&self) -> Result<Signal<Option<Message>>, RoomManagerError> {
        let val = self.state.borrow();
        match &*val {
            RoomState::Disconnected => Err(RoomManagerError::NotConnectedToRoom),
            RoomState::Connecting((connection, _, _)) => Ok(connection.message),

            RoomState::Connected(room_info) => Ok(room_info.connection.message),
        }
    }

    pub fn get_player_status(&self) -> Option<PlayerStatus> {
        if let Some(room_info) = self.room_info_signal.0.get_untracked() {
            Some(room_info.player_status)
        } else {
            None
        }
    }

    pub fn set_player_status(&self, player_status: PlayerStatus) {
        if let Some(mut room_info) = self.room_info_signal.0.get_untracked() {
            room_info.player_status = player_status;
            self.room_info_signal.1.set(Some(room_info));
        }
    }

    pub fn set_selected_video(&self, video_name: String) {
        if let Some(mut room_info) = self.room_info_signal.0.get_untracked() {
            if let Some(user) = room_info
                .users
                .iter_mut()
                .find(|u| u.id == room_info.user_id)
            {
                user.state = UserState::VideoSelected(video_name.clone());
                self.room_info_signal.1.set(Some(room_info));
                self.send_message(
                    common::message::ClientMessage::SelectedVideo(video_name),
                    crate::networking::room_manager::SendType::Reliable,
                );
            }
        }
    }

    pub fn send_message(&self, message: ClientMessage, send_type: SendType) {
        with_owner(self.owner, || {
            if let Some(player_id) = self
                .room_info_signal
                .0
                .with_untracked(|r| r.as_ref().map(|r| r.user_id))
            {
                if let RoomState::Connected(RoomConnectionInfo {
                    connection, socket, ..
                }) = &*self.state.borrow()
                {
                    match send_type {
                        SendType::Reliable => {
                            connection.send(Message::ClientMessage((player_id, message)));
                        }
                        SendType::UnReliablle => {
                            if let Some(socket) = socket.get_untracked() {
                                if socket.buffered_amount() < 5 {
                                    connection.send(Message::ClientMessage((player_id, message)));
                                }
                            } else {
                                warn!("Websocket is None");
                            }
                        }
                    }
                }
            }
        })
    }

    pub fn get_chat_signal(
        &self,
    ) -> Option<(
        ReadSignal<Option<(UserMeta, String)>>,
        StoredValue<Vec<(UserMeta, String)>>,
    )> {
        if let RoomState::Connected(RoomConnectionInfo {
            chat_history,
            chat_signal,
            ..
        }) = &*self.state.borrow()
        {
            Some((chat_signal.0, *chat_history))
        } else {
            None
        }
    }

    pub fn send_chat(&self, msg: String) {
        if msg.trim().is_empty() {
            return;
        }
        if let Some(user) = self.room_info_signal.0.with(|r| {
            r.as_ref()
                .and_then(|r| r.users.iter().find(|u| u.id == r.user_id).cloned())
        }) {
            {
                if let RoomState::Connected(RoomConnectionInfo { chat_signal, .. }) =
                    &*self.state.borrow()
                {
                    chat_signal.1.set(Some((user, msg.clone())));
                }
            }
            self.send_message(ClientMessage::Chat(msg), SendType::Reliable);
        }
    }

    pub async fn send_vc_request(
        &self,
        user: Uuid,
        video: bool,
        audio: bool,
    ) -> Result<(), JsValue> {
        let _ = get_media_stream(video, audio).await?;
        info!("Got permissions");
        self.send_message(
            ClientMessage::RequestCall(user, video, audio),
            SendType::Reliable,
        );
        self.vc_permission.update_value(|perms| {
            perms.insert(user, (video, audio));
        });
        info!("Sent vc request");
        Ok(())
    }

    pub async fn connect_audio_chat(
        &self,
        user: Uuid,
        video: bool,
        audio: bool,
    ) -> Result<(), JsValue> {
        info!("Connect host");
        let rtc_config_peer = if let RoomState::Connected(RoomConnectionInfo {
            rtc_config,

            rtc_peers,
            ..
        }) = &*self.state.borrow()
        {
            Some((*rtc_config, *rtc_peers))
        } else {
            None
        };

        let Some(room_info) = self.get_room_info().get_untracked() else {
            return Err(JsValue::from_str("Room not connected"));
        };

        if let Some((rtc_config, rtc_peers)) = rtc_config_peer {
            let ice_signal = self.ice_signal.0;
            let session_signal = self.sdp_signal.0;
            let owner = self.owner;

            let state = self.state.clone();
            let video_setter = self.video_chat_stream_signal.1;
            let audio_setter = self.audio_chat_stream_signal.1;
            let rtc_setter = self.rtc_signal.1;
            info!("Connect to user {user} self_id {}", room_info.user_id);
            let rm = self.clone();
            connect_to_user(
                room_info.user_id,
                user,
                &rtc_config.get_value(),
                video,
                audio,
                Callback::new(move |(id, stream)| {
                    video_setter.set(Some((id, stream)));
                }),
                Callback::new(move |(id, media)| {
                    audio_setter.set(Some((id, media)));
                }),
                Callback::new(move |(id, pc)| {
                    rtc_setter.set(Some((id, pc)));
                }),
                {
                    let rm = rm.clone();
                    Callback::new(move |ice| {
                        info!("Send ice {user} {ice:?}");
                        rm.send_message(
                            ClientMessage::ExchangeCandidate(user, ice),
                            SendType::Reliable,
                        );
                    })
                },
                {
                    let rm = rm.clone();
                    Callback::new(move |sdp| {
                        rm.send_message(
                            ClientMessage::SendSessionDesc(user, sdp),
                            SendType::Reliable,
                        );
                    })
                },
                ice_signal.into(),
                session_signal.into(),
                owner,
            )
            .await?;
            Ok(())
        } else {
            Err(JsValue::from_str("Room not connected"))
        }
    }
}

pub struct WebsocketContext<Tx>
where
    Tx: 'static,
{
    pub message: Signal<Option<Tx>>,
    send: WriteSignal<Option<Tx>>, // use Rc to make it easily cloneable
    _phantom: PhantomData<Tx>,
}

impl<Tx> WebsocketContext<Tx>
where
    Tx: 'static,
{
    pub fn new(message: Signal<Option<Tx>>, send: WriteSignal<Option<Tx>>) -> Self {
        Self {
            message,
            send,
            _phantom: PhantomData,
        }
    }

    // create a method to avoid having to use parantheses around the field
    #[inline(always)]
    pub fn send(&self, message: Tx) {
        self.send.set(Some(message));
    }
}
