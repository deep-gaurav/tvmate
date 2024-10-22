use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use codee::binary::BincodeSerdeCodec;
use common::{
    endpoints,
    message::{
        ClientMessage, Message, OfferReason, RTCSessionDesc, RtcConfig, UserJoined, UserLeft,
        VideoMeta,
    },
    params::{HostParams, JoinParams},
    PlayerStatus, UserMeta, UserState,
};
use leptos::{
    create_effect, create_rw_signal, create_signal, expect_context, logging::warn, store_value,
    with_owner, Callback, NodeRef, Owner, ReadSignal, RwSignal, Signal, SignalGet,
    SignalGetUntracked, SignalSet, SignalSetUntracked, SignalUpdate, SignalWith,
    SignalWithUntracked, StoredValue, WriteSignal,
};
use leptos_router::use_navigate;
use leptos_use::{
    core::ConnectionReadyState, use_websocket_with_options, UseWebSocketOptions, UseWebSocketReturn,
};
use thiserror::Error;
use tracing::info;
use uuid::Uuid;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{MediaStream, MediaStreamTrack, RtcPeerConnection, WebSocket};

use crate::{
    components::toaster::{Toast, Toaster},
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
    #[allow(clippy::type_complexity)]
    pub audio_chat_stream_signal: (
        ReadSignal<Option<(Uuid, Option<MediaStream>)>>,
        WriteSignal<Option<(Uuid, Option<MediaStream>)>>,
    ),
    #[allow(clippy::type_complexity)]
    pub video_chat_stream_signal: (
        ReadSignal<Option<(Uuid, Option<MediaStream>)>>,
        WriteSignal<Option<(Uuid, Option<MediaStream>)>>,
    ),
    #[allow(clippy::type_complexity)]
    pub rtc_signal: RwSignal<HashMap<Uuid, RtcPeerConnection>>,

    #[allow(clippy::type_complexity)]
    pub ice_signal: (
        ReadSignal<Option<(Uuid, String)>>,
        WriteSignal<Option<(Uuid, String)>>,
    ),
    #[allow(clippy::type_complexity)]
    pub sdp_signal: (
        ReadSignal<Option<(Uuid, RTCSessionDesc)>>,
        WriteSignal<Option<(Uuid, RTCSessionDesc)>>,
    ),
    pub vc_permission: StoredValue<HashMap<Uuid, (bool, bool)>>,

    pub self_video: RwSignal<Option<MediaStreamTrack>>,
    pub self_audio: RwSignal<Option<MediaStreamTrack>>,

    pub permission_request_signal: Signal<Option<(Uuid, bool, bool)>>,
    permission_request_sender: WriteSignal<Option<(Uuid, bool, bool)>>,

    pub share_video_signal: Signal<(
        Option<(Uuid, MediaStreamTrack)>,
        Option<(Uuid, MediaStreamTrack)>,
    )>,
    share_video_writer: WriteSignal<(
        Option<(Uuid, MediaStreamTrack)>,
        Option<(Uuid, MediaStreamTrack)>,
    )>,

    pub share_video_permission: Signal<Option<Uuid>>,
    share_video_permission_tx: WriteSignal<Option<Uuid>>,

    video_offer_type: StoredValue<OfferReason>,
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
    #[allow(clippy::type_complexity)]
    pub chat_signal: (
        ReadSignal<Option<(UserMeta, String)>>,
        WriteSignal<Option<(UserMeta, String)>>,
    ),
    pub rtc_config: StoredValue<RtcConfig>,
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
    Seek(f64, bool),
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
        let rtc_rtx = with_owner(owner, || create_rw_signal(HashMap::new()));
        let vc_permission = store_value(HashMap::new());

        let (permissions_rx, permissions_tx) = create_signal(None);

        let self_video = create_rw_signal(Option::<MediaStreamTrack>::None);
        let self_audio = create_rw_signal(None);

        let (share_video_rx, share_video_tx) = create_signal((None, None));

        let share_video_sig = create_signal(None);

        create_effect(move |_| {
            if let Some(vdo) = self_video.get() {
                info!("added self vdo id {}", vdo.id());
            }
        });

        let video_offer = store_value(OfferReason::VideoCall);

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
            rtc_signal: rtc_rtx,
            self_audio,
            self_video,
            share_video_signal: share_video_rx.into(),
            share_video_writer: share_video_tx,
            share_video_permission: share_video_sig.0.into(),
            share_video_permission_tx: share_video_sig.1,
            video_offer_type: video_offer,
        };
        with_owner(owner, {
            let rm = rm.clone();
            let state = rm.state.clone();
            move || {
                receive_peer_connections(
                    Callback::new(move |_| {
                        room_info_signal
                            .0
                            .with_untracked(|room| room.as_ref().map(|r: &RoomInfo| r.user_id))
                    }),
                    rtc_rtx,
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
                    Callback::new(move |(video, audio)| async move {
                        Self::get_video_audio_cb(video, audio, self_video, self_audio).await
                    }),
                    Callback::new(move |(user, stream)| {
                        video_tx.set(Some((user, stream)));
                    }),
                    Callback::new(move |(user, stream)| {
                        audio_tx.set(Some((user, stream)));
                    }),
                    share_video_tx,
                    video_offer,
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
                    Callback::new(move |_| {
                        self_video.update(|v| {
                            if let Some(v) = v {
                                v.stop();
                            }
                            *v = None
                        });
                        self_audio.update(|v| {
                            if let Some(v) = v {
                                v.stop();
                            }
                            *v = None
                        });
                    }),
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

    async fn get_video_audio_cb(
        video: bool,
        audio: bool,
        self_video: RwSignal<Option<MediaStreamTrack>>,
        self_audio: RwSignal<Option<MediaStreamTrack>>,
    ) -> (Option<MediaStreamTrack>, Option<MediaStreamTrack>) {
        let mut video_stream = None;
        let mut audio_stream = None;
        if video {
            video_stream = self_video.get_untracked();
        }
        if audio {
            audio_stream = self_audio.get_untracked();
        }

        let is_video_left = video && video_stream.is_none();
        let is_audio_left = audio && audio_stream.is_none();
        if is_audio_left || is_video_left {
            let remaining = get_media_stream(is_video_left, is_audio_left).await;

            match remaining {
                Ok(stream) => {
                    info!("Total tracks :{}", stream.get_tracks().length());
                    let audio = stream
                        .get_audio_tracks()
                        .get(0)
                        .dyn_into::<MediaStreamTrack>();
                    if let Ok(audio) = audio {
                        self_audio.update(|u| *u = Some(Clone::clone(&audio)));
                        audio_stream = Some(audio);
                    }

                    let video = stream
                        .get_video_tracks()
                        .get(0)
                        .dyn_into::<MediaStreamTrack>();
                    if let Ok(video) = video {
                        info!("Created vdo track 2 id {}", video.id());
                        self_video.update(|u| *u = Some(Clone::clone(&video)));

                        video_stream = Some(video);
                    }
                    if let Some(video) = &video_stream {
                        info!("Sending vdo track id {}", video.id());
                    }
                    (video_stream, audio_stream)
                }
                Err(err) => {
                    warn!("Could get media {err:?}");
                    (None, None)
                }
            }
        } else {
            if let Some(video) = &video_stream {
                info!("reusing vdo track id {}", video.id());
            }
            (video_stream, audio_stream)
        }
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
                    let share_permission_tx = self.share_video_permission_tx;

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

                                            let connection_info = RoomConnectionInfo {
                                                connection: unsafe { std::ptr::read(connection) },
                                                socket: *socket,
                                                ready_state: unsafe { std::ptr::read(ready_state) },
                                                chat_signal,
                                                chat_history,
                                                rtc_config,
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
                                    common::message::ServerMessage::Error(error) => {
                                        toaster.toast(Toast {
                                            message: error.into(),
                                            r#type: crate::components::toaster::ToastType::Failed,
                                        });
                                    }
                                },
                                Message::ClientMessage((from_user, message)) => match message {
                                    common::message::ClientMessage::SetVideoMeta(video_name) => {
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
                                    common::message::ClientMessage::Seek(time, before_seek) => {
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
                                            .set(Some(PlayerMessages::Seek(time, before_seek)));
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
                                        sdp_setter.set_untracked(None);
                                    }

                                    ClientMessage::ReceivedSessionDesc(sdp) => {
                                        info!("Received sdp from {from_user} {sdp:?}");
                                        sdp_setter.set(Some((from_user, sdp)));
                                        sdp_setter.set_untracked(None);
                                    }
                                    ClientMessage::RequestCall(_, video, audio) => {
                                        info!("Receivedd vc request");
                                        permission_request_notifier
                                            .set(Some((from_user, video, audio)));
                                    }
                                    ClientMessage::RequestVideoShare(_) => {
                                        share_permission_tx.set(Some(from_user));
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
                match &mut user.state {
                    UserState::VideoNotSelected => {
                        user.state = UserState::VideoSelected(VideoMeta {
                            name: video_name.clone(),
                            duration: None,
                        });
                    }
                    UserState::VideoSelected(video_meta) => {
                        video_meta.name = video_name.to_string();
                    }
                };
                self.send_message(
                    common::message::ClientMessage::SetVideoMeta(
                        user.state.as_video_selected().unwrap().clone(),
                    ),
                    crate::networking::room_manager::SendType::Reliable,
                );
                self.room_info_signal.1.set(Some(room_info));
            }
        }
    }

    pub fn set_video_duration(&self, duration: f64) {
        if let Some(mut room_info) = self.room_info_signal.0.get_untracked() {
            if let Some(user) = room_info
                .users
                .iter_mut()
                .find(|u| u.id == room_info.user_id)
            {
                match &mut user.state {
                    UserState::VideoNotSelected => {
                        warn!("Cannot set video duration without video");
                        return;
                    }
                    UserState::VideoSelected(video_meta) => video_meta.duration = Some(duration),
                };
                self.send_message(
                    common::message::ClientMessage::SetVideoMeta(
                        user.state.as_video_selected().unwrap().clone(),
                    ),
                    crate::networking::room_manager::SendType::Reliable,
                );
                self.room_info_signal.1.set(Some(room_info));
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

    #[allow(clippy::type_complexity)]
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
        let stream = get_media_stream(video, audio).await?;
        let audio_track = stream
            .get_audio_tracks()
            .get(0)
            .dyn_into::<MediaStreamTrack>();
        if let Ok(audio) = audio_track {
            self.self_audio.update(|v| *v = Some(audio));
        }

        let video_track = stream
            .get_video_tracks()
            .get(0)
            .dyn_into::<MediaStreamTrack>();
        if let Ok(video) = video_track {
            info!("Created vdo track 1 id {}", video.id());
            self.self_video.update(|v| *v = Some(video));
        }
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
        video_share: Option<NodeRef<leptos::html::Video>>,
        video: bool,
        audio: bool,
    ) -> Result<(), JsValue> {
        info!("Connect host");
        let rtc_config_peer =
            if let RoomState::Connected(RoomConnectionInfo { rtc_config, .. }) =
                &*self.state.borrow()
            {
                Some(*rtc_config)
            } else {
                None
            };

        let Some(room_info) = self.get_room_info().get_untracked() else {
            return Err(JsValue::from_str("Room not connected"));
        };

        if let Some(rtc_config) = rtc_config_peer {
            let ice_signal = self.ice_signal.0;
            let session_signal = self.sdp_signal.0;
            let owner = self.owner;

            let video_setter = self.video_chat_stream_signal.1;
            let audio_setter = self.audio_chat_stream_signal.1;

            let rtc_setter = self.rtc_signal;
            let self_video = self.self_video;
            let self_audio = self.self_audio;
            let share_setter = self.share_video_writer;
            let video_offer = self.video_offer_type;
            info!("Connect to user {user} self_id {}", room_info.user_id);
            let pc = self
                .rtc_signal
                .with_untracked(|peers| peers.get(&user).cloned());
            let rm = self.clone();
            if video_share.is_none() {
                self.vc_permission.update_value(|perms| {
                    perms.insert(user, (video, audio));
                });
            }
            connect_to_user(
                pc,
                video_share,
                room_info.user_id,
                user,
                &rtc_config.get_value(),
                video,
                audio,
                Callback::new(move |(video, audio)| async move {
                    Self::get_video_audio_cb(video, audio, self_video, self_audio).await
                }),
                Callback::new(move |(id, stream)| {
                    video_setter.set(Some((id, stream)));
                }),
                Callback::new(move |(id, media)| {
                    audio_setter.set(Some((id, media)));
                }),
                share_setter,
                video_offer,
                Callback::new(move |(id, pc)| {
                    rtc_setter.update(|peers| {
                        if let Some(pc) = pc {
                            peers.insert(id, pc);
                        } else {
                            peers.remove(&id);
                        }
                    });
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
                Callback::new(move |_| {
                    self_video.update(|v| {
                        if let Some(v) = v {
                            v.stop();
                        }
                        *v = None
                    });
                    self_audio.update(|v| {
                        if let Some(v) = v {
                            v.stop();
                        }
                        *v = None
                    });
                }),
                owner,
            )
            .await?;
            Ok(())
        } else {
            Err(JsValue::from_str("Room not connected"))
        }
    }

    pub async fn add_video_share(
        &self,
        user: Uuid,
        video: NodeRef<leptos::html::Video>,
    ) -> Result<(), JsValue> {
        info!("Try send video share");

        self.connect_audio_chat(user, Some(video), false, false)
            .await?;

        Ok(())
    }

    pub fn close_vc(&self, user: Uuid) -> Result<(), JsValue> {
        let Some(room_info) = self.get_room_info().get_untracked() else {
            return Err(JsValue::from_str("Room not connected"));
        };

        self.self_audio.update(|val| {
            if let Some(val) = val {
                val.stop();
            }
            *val = None;
        });

        self.self_video.update(|val| {
            if let Some(val) = val {
                val.stop();
            }
            *val = None;
        });

        self.audio_chat_stream_signal.1.set(Some((user, None)));
        self.video_chat_stream_signal.1.set(Some((user, None)));

        self.audio_chat_stream_signal
            .1
            .set(Some((room_info.user_id, None)));
        self.video_chat_stream_signal
            .1
            .set(Some((room_info.user_id, None)));

        self.rtc_signal.update(|peers| {
            peers.remove(&user);
        });

        Ok(())
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
