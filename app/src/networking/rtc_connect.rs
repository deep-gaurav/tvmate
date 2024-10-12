use std::{collections::HashMap, future::Future};

use common::message::{RTCSessionDesc, RtcConfig};
use leptos::{
    create_effect, store_value, with_owner, Callable, Callback, Owner, Signal, SignalGet,
};
use leptos_use::use_event_listener;
use tracing::{info, warn};
use uuid::Uuid;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    js_sys::{Array, JSON},
    window, MediaStream, MediaStreamConstraints, MediaStreamTrack, RtcConfiguration,
    RtcIceCandidate, RtcIceCandidateInit, RtcIceServer, RtcPeerConnection,
    RtcPeerConnectionIceEvent, RtcPeerConnectionState, RtcSdpType, RtcSessionDescriptionInit,
    RtcTrackEvent,
};

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
) -> Result<(), JsValue> {
    let ms = MediaStream::new()?;

    if let Some(track) = audio {
        info!("Add Audio track");
        pc.add_track(&track, &ms, &Array::new());
    }

    if let Some(track) = video {
        info!("Add Video track");
        pc.add_track(&track, &ms, &Array::new());
    }

    Ok(())
}

pub async fn connect_to_user<F>(
    self_id: Uuid,
    user: Uuid,
    rtc_config: &RtcConfig,
    video: bool,
    audio: bool,
    self_video_cb: Callback<(bool, bool), F>,

    video_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
    audio_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
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
    let pc = connect_rtc(rtc_config)?;

    let is_closed = store_value(false);
    let pending_ice = store_value(Some(vec![]));

    with_owner(owner, || {
        let _ = use_event_listener(
            pc.clone(),
            leptos::ev::Custom::<RtcTrackEvent>::new("track"),
            move |ev| {
                info!("Host: ev - track");
                let track = ev.track();
                if let Ok(stream) = MediaStream::new_with_tracks(&Array::of1(&track)) {
                    if track.kind() == "audio" {
                        audio_media_setter.call((user, Some(stream)));
                    } else {
                        video_media_setter.call((user, Some(stream)));
                    }
                }
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
                        RtcPeerConnectionState::Closed | RtcPeerConnectionState::Disconnected => {
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

    let offer = wasm_bindgen_futures::JsFuture::from(pc.create_offer()).await?;
    let offer = offer.unchecked_into::<RtcSessionDescriptionInit>();
    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&offer)).await?;

    session_callback.call(RTCSessionDesc {
        typ: JsValue::from(offer.get_type())
            .as_string()
            .expect("sdp type not string"),
        sdp: offer.get_sdp().expect("No sdp"),
    });

    with_owner(owner, || {
        create_effect({
            let pc = pc.clone();
            move |_| {
                if is_closed.get_value() {
                    return;
                }
                if let Some((id, rtcsession_desc)) = session_signal.get() {
                    info!("Host: received sdp {id} {rtcsession_desc:?}");
                    if id == user {
                        if let Some(rtc_sdp) =
                            RtcSdpType::from_js_value(&JsValue::from_str(&rtcsession_desc.typ))
                        {
                            let rtc_sdp = RtcSessionDescriptionInit::new(rtc_sdp);
                            rtc_sdp.set_sdp(&rtcsession_desc.sdp);
                            leptos::spawn_local({
                                let pc = pc.clone();
                                async move {
                                    let _ = wasm_bindgen_futures::JsFuture::from(
                                        pc.set_remote_description(&rtc_sdp),
                                    )
                                    .await;
                                    pending_ice.update_value(|p_ice|{
                                        if let Some(p_ice) = p_ice.take() {
                                            for candidate in p_ice {
                                                let _ = pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(
                                                    &candidate,
                                                ));
                                            }
                                        }
                                    });
                                }
                            });
                        }
                    }
                }
            }
        });
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
    });

    Ok(())
}

pub fn receive_peer_connections<F>(
    self_id: Callback<(), Option<Uuid>>,
    rtc_config: Callback<(), Option<RtcConfig>>,

    permissions_callback: Callback<Uuid, (bool, bool)>,
    self_video_cb: Callback<(bool, bool), F>,

    video_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
    audio_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
    rtc_setter: Callback<(Uuid, Option<RtcPeerConnection>), ()>,

    ice_callback: Callback<(Uuid, String)>,
    session_callback: Callback<(Uuid, RTCSessionDesc)>,

    ice_signal: Signal<Option<(Uuid, String)>>,
    session_signal: Signal<Option<(Uuid, RTCSessionDesc)>>,

    close_self: Callback<()>,

    owner: Owner,
) where
    F: Future<Output = (Option<MediaStreamTrack>, Option<MediaStreamTrack>)> + 'static,
{
    let peers = store_value(HashMap::<Uuid, RtcPeerConnection>::new());
    let pending_candidates = store_value(HashMap::<Uuid, Vec<RtcIceCandidateInit>>::new());

    create_effect(move |_| {
        if let Some((from_user, candidate)) = ice_signal.get() {
            if let Ok(candidate) = deserialize_candidate(&candidate) {
                if let Some(pc) = peers.with_value(|peers| peers.get(&from_user).cloned()) {
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
            info!("Got sdp from {from_user} starting connection");
            let Ok(offer_type) =
                RtcSdpType::from_js_value(&JsValue::from_str(&rtcsession_desc.typ))
                    .ok_or(JsValue::from_str("cannot convert sdp type"))
            else {
                warn!("Cant get offer type");
                return;
            };
            if offer_type != RtcSdpType::Offer {
                info!("Ignoring {offer_type:?} as it's not offer");
                return;
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
                if !video && !audio {
                    warn!("permissions not gived for video and audio");
                    return;
                }
                let pc = match connect_rtc(&rtc_config) {
                    Ok(pc) => pc,
                    Err(er) => {
                        warn!("Cant create pc {er:?}");
                        return;
                    }
                };

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
                                        peers.update_value(|p| {
                                            info!("disconnected, remove pc");
                                            p.remove(&from_user);
                                        });

                                        rtc_setter.call((from_user, None));
                                        video_media_setter.call((from_user, None));
                                        audio_media_setter.call((from_user, None));
                                        video_media_setter.call((self_id, None));
                                        audio_media_setter.call((self_id, None));

                                        pc.close();
                                        close_self.call(());
                                    }
                                    RtcPeerConnectionState::Connected => {
                                        rtc_setter.call((from_user, Some(pc.clone())));
                                    }
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
                            let track = ev.track();
                            if let Ok(stream) = MediaStream::new_with_tracks(&Array::of1(&track)) {
                                if track.kind() == "audio" {
                                    audio_media_setter.call((from_user, Some(stream)));
                                } else {
                                    video_media_setter.call((from_user, Some(stream)));
                                }
                            }
                        },
                    );
                });

                match accept_peer_connection(
                    self_id,
                    &pc,
                    rtcsession_desc,
                    video,
                    audio,
                    self_video_cb,
                    video_media_setter,
                    audio_media_setter,
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

                        peers.update_value(|p| {
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

async fn accept_peer_connection<F>(
    self_id: Uuid,

    pc: &RtcPeerConnection,
    rtc_session_desc: RTCSessionDesc,

    video: bool,
    audio: bool,
    self_video_cb: Callback<(bool, bool), F>,

    video_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
    audio_media_setter: Callback<(Uuid, Option<MediaStream>), ()>,
) -> Result<RTCSessionDesc, JsValue>
where
    F: Future<Output = (Option<MediaStreamTrack>, Option<MediaStreamTrack>)> + 'static,
{
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

    add_media_tracks(pc, video_track, audio_track).await?;

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
    })
}
