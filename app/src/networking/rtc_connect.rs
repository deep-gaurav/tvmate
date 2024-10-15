use std::{collections::HashMap, future::Future};

use common::message::{OfferReason, RTCSessionDesc, RtcConfig};
use leptos::{
    create_effect, store_value, with_owner, Callable, Callback, NodeRef, Owner, RwSignal, Signal,
    SignalGet, SignalUpdate, SignalUpdateUntracked, SignalWithUntracked, StoredValue, WriteSignal,
};
use leptos_use::use_event_listener;
use tracing::{info, warn};
use uuid::Uuid;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    js_sys::{Array, JSON},
    window, Event, HtmlMediaElement, MediaStream, MediaStreamConstraints, MediaStreamTrack,
    RtcConfiguration, RtcIceCandidate, RtcIceCandidateInit, RtcIceGatheringState, RtcIceServer,
    RtcPeerConnection, RtcPeerConnectionIceEvent, RtcPeerConnectionState, RtcRtpSender, RtcSdpType,
    RtcSessionDescriptionInit, RtcTrackEvent,
};

use crate::web_glue::HtmlMediaElement2;

pub fn connect_rtc(rtc_config: &RtcConfig) -> Result<RtcPeerConnection, JsValue> {
    RtcPeerConnection::new_with_configuration(&{
        let config = RtcConfiguration::new();
        config.set_ice_servers(&{
            let array = Array::new();
            array.push(&JsValue::from({
                let ice_server = RtcIceServer::new();
                ice_server.set_urls(&JsValue::from_str(&rtc_config.stun));
                ice_server
            }));
            array.push(&JsValue::from({
                let ice_server = RtcIceServer::new();
                ice_server.set_urls(&JsValue::from_str(&rtc_config.turn));
                ice_server.set_username(&rtc_config.turn_user);
                ice_server.set_credential(&rtc_config.turn_creds);
                ice_server
            }));
            JsValue::from(array)
        });
        config
    })
}

pub fn serialize_candidate(candidate: RtcIceCandidate) -> Result<String, JsValue> {
    JSON::stringify(&candidate.to_json()).map(|s| s.into())
}

pub fn deserialize_candidate(candidate: &str) -> Result<RtcIceCandidateInit, JsValue> {
    let obj = JSON::parse(candidate)?;
    Ok(obj.unchecked_into())
}

pub async fn get_media_stream(video: bool, audio: bool) -> Result<MediaStream, JsValue> {
    let user_media = window()
        .unwrap()
        .navigator()
        .media_devices()
        .expect("No Media Devices")
        .get_user_media_with_constraints(&{
            let constraints = MediaStreamConstraints::new();
            constraints.set_audio(&JsValue::from_bool(audio));
            constraints.set_video(&JsValue::from_bool(video));
            constraints
        })?;
    let media_stream = wasm_bindgen_futures::JsFuture::from(user_media)
        .await?
        .dyn_into::<MediaStream>()?;
    Ok(media_stream)
}

pub async fn add_media_tracks(
    pc: &RtcPeerConnection,
    video: Option<MediaStreamTrack>,
    audio: Option<MediaStreamTrack>,
) -> Result<(bool, bool), JsValue> {
    let ms = MediaStream::new()?;

    let mut send_tracks = vec![];

    for sender in pc.get_senders() {
        if let Ok(sender) = sender.dyn_into::<RtcRtpSender>() {
            if let Some(send_track) = sender.track() {
                send_tracks.push(send_track.id());
            }
        }
    }

    let (mut audio_set, mut video_set) = (false, false);
    if let Some(track) = audio {
        info!("Add Audio track");
        if !send_tracks.contains(&track.id()) {
            pc.add_track(&track, &ms, &Array::new());
            audio_set = true;
        }
    }

    if let Some(track) = video {
        info!("Add Video track");
        if !send_tracks.contains(&track.id()) {
            pc.add_track(&track, &ms, &Array::new());
            video_set = true
        }
    }

    Ok((video_set, audio_set))
}

async fn create_send_offer(
    pc: RtcPeerConnection,
    share_tracks: Option<Vec<String>>,
) -> Result<RTCSessionDesc, JsValue> {
    let offer = wasm_bindgen_futures::JsFuture::from(pc.create_offer()).await?;
    let offer = offer.unchecked_into::<RtcSessionDescriptionInit>();
    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&offer)).await?;

    Ok(RTCSessionDesc {
        typ: JsValue::from(offer.get_type())
            .as_string()
            .expect("sdp type not string"),
        sdp: offer.get_sdp().expect("No sdp"),
        reason: share_tracks
            .map(|tracks| OfferReason::VideoShare(tracks))
            .unwrap_or(common::message::OfferReason::VideoCall),
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn connect_to_user<F>(
    pc: Option<RtcPeerConnection>,
    video_node: Option<NodeRef<leptos::html::Video>>,
    self_id: Uuid,
    user: Uuid,
    rtc_config: &RtcConfig,
    video: bool,
    audio: bool,
    self_video_cb: Callback<(bool, bool), F>,

    video_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
    audio_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
    video_share_setter: WriteSignal<(Option<MediaStreamTrack>, Option<MediaStreamTrack>)>,
    video_offer_tyoe: StoredValue<OfferReason>,

    rtc_setter: Callback<(Uuid, Option<RtcPeerConnection>), ()>,

    ice_callback: Callback<String>,
    session_callback: Callback<RTCSessionDesc>,

    ice_signal: Signal<Option<(Uuid, String)>>,
    session_signal: Signal<Option<(Uuid, RTCSessionDesc)>>,

    close_self: Callback<()>,
    owner: Owner,
) -> Result<(), JsValue>
where
    F: Future<Output = (Option<MediaStreamTrack>, Option<MediaStreamTrack>)> + 'static,
{
    info!("Host user");
    let is_connected = pc.is_some();
    let pc = if let Some(pc) = pc {
        pc
    } else {
        connect_rtc(rtc_config)?
    };

    let is_closed = store_value(false);
    let pending_ice = store_value(Some(vec![]));

    let video_tracks = store_value(HashMap::<Uuid, Vec<String>>::new());

    if !is_connected {
        with_owner(owner, || {
            let _ = use_event_listener(
                pc.clone(),
                leptos::ev::Custom::<RtcTrackEvent>::new("track"),
                move |ev| {
                    info!("Host: ev - track");
                    let track = ev.track();
                    let video_track_ids = video_tracks
                        .with_value(|v| v.get(&user).cloned())
                        .unwrap_or_default();
                    if let Ok(stream) = MediaStream::new_with_tracks(&Array::of1(&track)) {
                        if track.kind() == "audio" {
                            {
                                if video_track_ids.contains(&track.id()) {
                                    video_share_setter.update(|(video, audio)| {
                                        info!("Add shared audio");
                                        *audio = Some(track);
                                    });
                                } else {
                                    audio_media_setter.call((user, Some(stream)));
                                }
                            }
                        } else {
                            {
                                if video_track_ids.contains(&track.id()) {
                                    video_share_setter.update(|(video, audio)| {
                                        info!("Add shared video");
                                        *video = Some(track);
                                    });
                                } else {
                                    video_media_setter.call((user, Some(stream)));
                                }
                            }
                        }
                    }
                },
            );
        });
    }

    if !is_connected {
        with_owner(owner, || {
            let _ = use_event_listener(
                pc.clone(),
                leptos::ev::Custom::<leptos::ev::Event>::new("connectionstatechange"),
                {
                    let pc = pc.clone();
                    move |_| {
                        let connection = pc.connection_state();
                        info!("State changed to {connection:?}");

                        match connection {
                            RtcPeerConnectionState::Closed
                            | RtcPeerConnectionState::Disconnected => {
                                rtc_setter.call((user, None));
                                video_media_setter.call((user, None));
                                audio_media_setter.call((user, None));
                                video_media_setter.call((self_id, None));
                                audio_media_setter.call((self_id, None));
                                close_self.call(());

                                pc.close();
                                is_closed.set_value(true);
                            }
                            RtcPeerConnectionState::Connected => {
                                rtc_setter.call((user, Some(pc.clone())));
                            }
                            _ => {}
                        }
                    }
                },
            );
        });
    }

    if !is_connected {
        with_owner(owner, || {
            let _ = use_event_listener(
                pc.clone(),
                leptos::ev::Custom::<RtcPeerConnectionIceEvent>::new("icecandidate"),
                move |ev| {
                    info!("Host: ev - ice");
                    if let Some(candidate) = ev.candidate() {
                        if let Ok(candidate) = serialize_candidate(candidate) {
                            info!("Sending ice host");
                            ice_callback.call(candidate);
                        } else {
                            warn!("Cant serialize candidate")
                        }
                    }
                },
            );
        });
    }

    if !is_connected {
        with_owner(owner, || {
            let pc = pc.clone();
            let _ = use_event_listener(
                pc.clone(),
                leptos::ev::Custom::<Event>::new("icegatheringstatechange"),
                move |_| {
                    rtc_setter.call((user, Some(pc.clone())));

                    if pc.ice_gathering_state() == RtcIceGatheringState::Gathering {
                        pending_ice.update_value(|p_ice| {
                            if let Some(p_ice) = p_ice.take() {
                                for candidate in p_ice {
                                    let _ = pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(
                                        Some(&candidate),
                                    );
                                }
                            }
                        });
                    }
                },
            );
        });
        with_owner(owner, || {
            let pc = pc.clone();
            let _ = use_event_listener(
                pc.clone(),
                leptos::ev::Custom::<Event>::new("negotiationneeded"),
                move |_| {
                    leptos::spawn_local({
                        let pc = pc.clone();
                        async move {
                            let tracks = if let OfferReason::VideoShare(trakcs) =
                                video_offer_tyoe.get_value()
                            {
                                Some(trakcs)
                            } else {
                                None
                            };
                            let offer = create_send_offer(pc, tracks).await;
                            match offer {
                                Ok(offer) => {
                                    video_offer_tyoe.set_value(OfferReason::VideoCall);
                                    session_callback.call(offer);
                                }
                                Err(_) => {
                                    warn!("Renegotiate offer create failed")
                                }
                            }
                        }
                    });
                },
            );
        })
    }

    if let Some(video_node) = video_node {
        let offer = add_video_share_track(&pc, video_node).await?;

        video_offer_tyoe.update_value(|c| *c = OfferReason::VideoShare(offer));
    } else {
        let (video_track, audio_track) = self_video_cb.call((video, audio)).await;

        if video && video_track.is_none() {
            return Err(JsValue::from_str("Cannot get video"));
        }

        if audio && audio_track.is_none() {
            return Err(JsValue::from_str("Cannot get video"));
        }

        if let Some(audio) = audio_track.clone() {
            if let Ok(audio_stream) = MediaStream::new_with_tracks(&Array::of1(&audio)) {
                info!("Host: set audio");
                audio_media_setter.call((self_id, Some(audio_stream)));
            } else {
                warn!("Host: Cant make audio");
            }
        }
        if let Some(video) = video_track.clone() {
            if let Ok(video_stream) = MediaStream::new_with_tracks(&Array::of1(&video)) {
                info!("Host: set video");
                video_media_setter.call((self_id, Some(video_stream)));
            } else {
                warn!("Host: Cant make video");
            }
        }

        add_media_tracks(&pc, video_track, audio_track).await?;
    }

    with_owner(owner, || {
        if !is_connected {
            create_effect({
                let pc = pc.clone();
                move |_| {
                    if is_closed.get_value() {
                        return;
                    }
                    if let Some((_, rtcsession_desc)) = session_signal.get() {
                        if let OfferReason::VideoShare(mut track_ids) =
                            rtcsession_desc.reason.clone()
                        {
                            info!("video share offer {track_ids:?}");
                            video_tracks.update_value(|tracks| {
                                if let Some(tracks) = tracks.get_mut(&user) {
                                    tracks.append(&mut track_ids);
                                } else {
                                    tracks.insert(user, track_ids);
                                }
                            });
                        }
                    }
                }
            });
        }
        if !is_connected {
            create_effect(move |_| {
                if is_closed.get_value() {
                    return;
                }
                if let Some((id, candidate)) = ice_signal.get() {
                    info!("Host: received ice {id} {candidate}");

                    if id == user {
                        if let Ok(candidate) = deserialize_candidate(&candidate) {
                            info!("Add ice, is_closed {}", is_closed.get_value());
                            if pending_ice.with_value(|p| p.is_some()) {
                                pending_ice.update_value(|p| {
                                    if let Some(p) = p {
                                        p.push(candidate);
                                    }
                                });
                            } else {
                                let _ = pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(
                                    &candidate,
                                ));
                            }
                        } else {
                            warn!("Cant deserialize candidate")
                        }
                    }
                }
            });
        }
    });

    Ok(())
}

pub async fn add_video_share_track(
    pc: &RtcPeerConnection,
    video_ref: NodeRef<leptos::html::Video>,
) -> Result<Vec<String>, JsValue> {
    let Some(video) = video_ref.get_untracked() else {
        return Err(JsValue::from_str("cant get video"));
    };
    let media_el: &HtmlMediaElement = video.as_ref();
    info!("Got media element");
    let media_el2: &HtmlMediaElement2 = media_el.unchecked_ref();

    let stream = media_el2.capture_stream()?;
    let mut track_ids = vec![];
    for track in stream.get_tracks() {
        if let Ok(track) = track.dyn_into::<MediaStreamTrack>() {
            pc.add_track_0(&track, &stream);
            track_ids.push(track.id());
        }
    }
    Ok(track_ids)
}

#[allow(clippy::too_many_arguments)]
pub fn receive_peer_connections<F>(
    self_id: Callback<(), Option<Uuid>>,
    peers: RwSignal<HashMap<Uuid, RtcPeerConnection>>,
    rtc_config: Callback<(), Option<RtcConfig>>,

    permissions_callback: Callback<Uuid, (bool, bool)>,
    self_video_cb: Callback<(bool, bool), F>,

    video_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
    audio_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,

    video_share_setter: WriteSignal<(Option<MediaStreamTrack>, Option<MediaStreamTrack>)>,
    video_offer_type: StoredValue<OfferReason>,

    ice_callback: Callback<(Uuid, String)>,
    session_callback: Callback<(Uuid, RTCSessionDesc)>,

    ice_signal: Signal<Option<(Uuid, String)>>,
    session_signal: Signal<Option<(Uuid, RTCSessionDesc)>>,

    close_self: Callback<()>,

    owner: Owner,
) where
    F: Future<Output = (Option<MediaStreamTrack>, Option<MediaStreamTrack>)> + 'static,
{
    let pending_candidates = store_value(HashMap::<Uuid, Vec<RtcIceCandidateInit>>::new());

    let video_tracks = store_value(HashMap::<Uuid, Vec<String>>::new());

    create_effect(move |_| {
        if let Some((from_user, candidate)) = ice_signal.get() {
            if let Ok(candidate) = deserialize_candidate(&candidate) {
                if let Some(pc) = peers.with_untracked(|peers| peers.get(&from_user).cloned()) {
                    info!("add ice to pc");
                    let _ = pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate));
                } else {
                    pending_candidates.update_value(|ice| {
                        let ice_queue = ice.entry(from_user).or_default();
                        ice_queue.push(candidate);
                    });
                }
            } else {
                warn!("Cant deserialize candidate")
            }
        }
    });

    create_effect(move |_| {
        if let Some((from_user, rtcsession_desc)) = session_signal.get() {
            let pc = if let Some((
                pc,
                RtcPeerConnectionState::Connected
                | RtcPeerConnectionState::Connecting
                | RtcPeerConnectionState::New,
            )) = peers.with_untracked(|pc| {
                pc.get(&from_user)
                    .map(|p| (p.clone(), p.connection_state()))
            }) {
                Some(pc)
            } else {
                None
            };

            info!("Got sdp from {from_user} starting connection");

            let Ok(offer_type) =
                RtcSdpType::from_js_value(&JsValue::from_str(&rtcsession_desc.typ))
                    .ok_or(JsValue::from_str("cannot convert sdp type"))
            else {
                warn!("Cant get offer type");
                return;
            };
            if offer_type == RtcSdpType::Answer {
                if let Some(pc) = pc {
                    let rtc_sdp = RtcSessionDescriptionInit::new(offer_type);
                    rtc_sdp.set_sdp(&rtcsession_desc.sdp);
                    leptos::spawn_local(async move {
                        info!("Setting remoter answer and exiting in receive_peer_connections");
                        if let Err(err) = wasm_bindgen_futures::JsFuture::from(
                            pc.set_remote_description(&rtc_sdp),
                        )
                        .await
                        {
                            warn!("Cannot set answer {err:?}");
                        }
                    });
                } else {
                    warn!("Received answer but pc not present");
                }
                return;
            }
            if offer_type != RtcSdpType::Offer {
                info!("Ignoring {offer_type:?} as it's not offer");
                return;
            }

            if let OfferReason::VideoShare(mut track_ids) = rtcsession_desc.reason.clone() {
                video_tracks.update_value(|tracks| {
                    if let Some(tracks) = tracks.get_mut(&from_user) {
                        tracks.append(&mut track_ids);
                    } else {
                        tracks.insert(from_user, track_ids);
                    }
                });
            }
            let Some(self_id) = self_id.call(()) else {
                return;
            };
            info!("Self id {self_id}");
            let Some(rtc_config) = rtc_config.call(()) else {
                return;
            };
            leptos::spawn_local(async move {
                let (video, audio) = permissions_callback.call(from_user);
                if rtcsession_desc.reason == OfferReason::VideoCall && !video && !audio {
                    warn!("permissions not gived for video and audio");
                    return;
                }
                let does_connection_exist = pc.is_some();
                let pc = match pc {
                    Some(pc) => pc,
                    None => match connect_rtc(&rtc_config) {
                        Ok(pc) => pc,
                        Err(er) => {
                            warn!("Cant create pc {er:?}");
                            return;
                        }
                    },
                };

                if !does_connection_exist {
                    with_owner(owner, || {
                        let pc = pc.clone();
                        let _ = use_event_listener(
                            pc.clone(),
                            leptos::ev::Custom::<Event>::new("negotiationneeded"),
                            move |_| {
                                leptos::spawn_local({
                                    let pc = pc.clone();
                                    async move {
                                        let tracks = if let OfferReason::VideoShare(trakcs) =
                                            video_offer_type.get_value()
                                        {
                                            Some(trakcs)
                                        } else {
                                            None
                                        };
                                        let offer = create_send_offer(pc, tracks).await;
                                        match offer {
                                            Ok(offer) => {
                                                video_offer_type.set_value(OfferReason::VideoCall);
                                                session_callback.call((from_user, offer));
                                            }
                                            Err(_) => {
                                                warn!("Renegotiate offer create failed")
                                            }
                                        }
                                    }
                                });
                            },
                        );
                    });

                    with_owner(owner, || {
                        let _ = use_event_listener(
                            pc.clone(),
                            leptos::ev::Custom::<leptos::ev::Event>::new("connectionstatechange"),
                            {
                                let pc = pc.clone();
                                move |_| {
                                    let connection = pc.connection_state();
                                    info!("State changed to {connection:?}");
                                    match connection {
                                        RtcPeerConnectionState::Closed
                                        | RtcPeerConnectionState::Disconnected => {
                                            peers.update(|p| {
                                                info!("disconnected, remove pc");
                                                p.remove(&from_user);
                                            });

                                            video_media_setter.call((from_user, None));
                                            audio_media_setter.call((from_user, None));
                                            video_media_setter.call((self_id, None));
                                            audio_media_setter.call((self_id, None));

                                            pc.close();
                                            close_self.call(());
                                        }
                                        RtcPeerConnectionState::Connected => {}
                                        _ => {}
                                    }
                                }
                            },
                        );
                    });

                    with_owner(owner, || {
                        let _ = use_event_listener(
                            pc.clone(),
                            leptos::ev::Custom::<RtcTrackEvent>::new("track"),
                            move |ev| {
                                info!("Peer: ev: track");
                                let video_track_ids = video_tracks
                                    .with_value(|tracks| tracks.get(&from_user).cloned())
                                    .unwrap_or_default();

                                let track = ev.track();

                                info!("Tracks {:?}", video_track_ids);
                                info!("Tracl {:?}", track.id());
                                if let Ok(stream) =
                                    MediaStream::new_with_tracks(&Array::of1(&track))
                                {
                                    if track.kind() == "audio" {
                                        {
                                            if video_track_ids.contains(&track.id()) {
                                                video_share_setter.update(|(video, audio)| {
                                                    *audio = Some(track);
                                                });
                                            } else {
                                                audio_media_setter.call((from_user, Some(stream)));
                                            }
                                        }
                                    } else {
                                        {
                                            if video_track_ids.contains(&track.id()) {
                                                video_share_setter.update(|(video, audio)| {
                                                    *video = Some(track);
                                                });
                                            } else {
                                                video_media_setter.call((from_user, Some(stream)));
                                            }
                                        }
                                    }
                                }
                            },
                        );
                    });
                }

                match accept_peer_connection(
                    self_id,
                    &pc,
                    rtcsession_desc,
                    video,
                    audio,
                    self_video_cb,
                    video_media_setter,
                    audio_media_setter,
                    does_connection_exist,
                )
                .await
                {
                    Ok(answer) => {
                        if let Some(candidates) =
                            pending_candidates.with_value(|pc| pc.get(&from_user).cloned())
                        {
                            pending_candidates.update_value(|pc| {
                                pc.remove(&from_user);
                            });
                            for candidate in candidates {
                                info!("Add ice to pc");
                                let _ = pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(
                                    &candidate,
                                ));
                            }
                        }

                        with_owner(owner, || {
                            let _ = use_event_listener(
                                pc.clone(),
                                leptos::ev::Custom::<RtcPeerConnectionIceEvent>::new(
                                    "icecandidate",
                                ),
                                move |ev| {
                                    if let Some(candidate) = ev.candidate() {
                                        if let Ok(candidate) = serialize_candidate(candidate) {
                                            info!("Sending ice receiver");
                                            ice_callback.call((from_user, candidate));
                                        } else {
                                            warn!("Cant serialize candidate")
                                        }
                                    }
                                },
                            );
                        });

                        peers.update(|p| {
                            p.insert(from_user, pc);
                        });
                        session_callback.call((from_user, answer));
                    }

                    Err(err) => {
                        warn!("Cant receive connection {err:?}");
                    }
                }
            });
        }
    });
}

#[allow(clippy::too_many_arguments)]
async fn accept_peer_connection<F>(
    self_id: Uuid,

    pc: &RtcPeerConnection,
    rtc_session_desc: RTCSessionDesc,

    video: bool,
    audio: bool,
    self_video_cb: Callback<(bool, bool), F>,

    video_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
    audio_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
    is_connection_reuse: bool,
) -> Result<RTCSessionDesc, JsValue>
where
    F: Future<Output = (Option<MediaStreamTrack>, Option<MediaStreamTrack>)> + 'static,
{
    let mut is_new_track_added = false;
    if rtc_session_desc.reason == OfferReason::VideoCall {
        info!("Get local audio {audio} video {video}");
        let (video_track, audio_track) = self_video_cb.call((video, audio)).await;

        info!(
            "Received local audio :{} video: {}",
            audio_track.is_some(),
            video_track.is_some()
        );

        if video && video_track.is_none() {
            return Err(JsValue::from_str("Cannot get video"));
        }

        if audio && audio_track.is_none() {
            return Err(JsValue::from_str("Cannot get video"));
        }

        if let Some(audio) = audio_track.clone() {
            if let Ok(audio_stream) = MediaStream::new_with_tracks(&Array::of1(&audio)) {
                audio_media_setter.call((self_id, Some(audio_stream)));
            }
        }
        if let Some(video) = video_track.clone() {
            if let Ok(video_stream) = MediaStream::new_with_tracks(&Array::of1(&video)) {
                video_media_setter.call((self_id, Some(video_stream)));
            }
        }

        let (video_set, audio_set) = add_media_tracks(pc, video_track, audio_track).await?;
        is_new_track_added = video_set || audio_set;
    }

    info!("Accepting answer, no new tracks added or first connection");
    let offer_type = RtcSdpType::from_js_value(&JsValue::from_str(&rtc_session_desc.typ))
        .ok_or(JsValue::from_str("cannot convert sdp type"))?;
    let rtc_sdp = RtcSessionDescriptionInit::new(offer_type);
    rtc_sdp.set_sdp(&rtc_session_desc.sdp);
    wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&rtc_sdp)).await?;

    let answer = wasm_bindgen_futures::JsFuture::from(pc.create_answer()).await?;
    let answer = answer.unchecked_into::<RtcSessionDescriptionInit>();

    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&answer)).await?;

    Ok(RTCSessionDesc {
        typ: JsValue::from(answer.get_type())
            .as_string()
            .expect("sdp type not string"),
        sdp: answer.get_sdp().expect("No sdp"),
        reason: rtc_session_desc.reason,
    })
}
