use std::collections::HashMap;

use common::message::{RTCSessionDesc, RtcConfig};
use leptos::{
    create_effect, store_value, with_owner, Callable, Callback, Owner, Signal, SignalGet,
    WriteSignal,
};
use leptos_use::use_event_listener;
use tracing::{info, warn};
use uuid::Uuid;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    js_sys::{Array, JSON},
    window, MediaStream, MediaStreamConstraints, MediaStreamTrack, RtcConfiguration,
    RtcIceCandidate, RtcIceCandidateInit, RtcIceServer, RtcPeerConnection,
    RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescriptionInit, RtcTrackEvent,
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
    video: bool,
    audio: bool,
) -> Result<MediaStream, JsValue> {
    let media_stream = get_media_stream(video, audio).await?;

    for track in media_stream.get_audio_tracks() {
        info!("Add Audio track");
        pc.add_track(&track.dyn_into()?, &media_stream, &Array::new());
    }

    for track in media_stream.get_video_tracks() {
        info!("Add Video track");
        pc.add_track(&track.dyn_into()?, &media_stream, &Array::new());
    }

    let offer = wasm_bindgen_futures::JsFuture::from(pc.create_offer()).await?;
    let offer = offer.unchecked_into::<RtcSessionDescriptionInit>();

    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&offer)).await?;

    Ok(media_stream)
}

pub async fn connect_to_user(
    self_id: Uuid,
    user: Uuid,
    rtc_config: &RtcConfig,
    video: bool,
    audio: bool,
    video_media_setter: Callback<(Uuid, MediaStream), ()>,
    audio_media_setter: Callback<(Uuid, MediaStream), ()>,

    ice_callback: Callback<String>,
    session_callback: Callback<RTCSessionDesc>,

    ice_signal: Signal<Option<(Uuid, String)>>,
    session_signal: Signal<Option<(Uuid, RTCSessionDesc)>>,

    owner: Owner,
) -> Result<(), JsValue> {
    info!("Host user");
    let pc = connect_rtc(rtc_config)?;

    with_owner(owner, || {
        let _ = use_event_listener(
            pc.clone(),
            leptos::ev::Custom::<RtcTrackEvent>::new("track"),
            move |ev| {
                info!("Host: ev - track");
                let track = ev.track();
                if let Ok(stream) = MediaStream::new_with_tracks(&Array::of1(&track)) {
                    if track.kind() == "audio" {
                        audio_media_setter.call((user, stream));
                    } else {
                        video_media_setter.call((user, stream));
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

    let local_stream = add_media_tracks(&pc, video, audio).await?;

    if let Ok(audio) = local_stream
        .get_audio_tracks()
        .get(0)
        .dyn_into::<MediaStreamTrack>()
    {
        if let Ok(audio_stream) = MediaStream::new_with_tracks(&Array::of1(&audio)) {
            info!("Host: set audio");
            audio_media_setter.call((self_id, audio_stream));
        } else {
            warn!("Host: Cant make audio");
        }
    }
    if let Ok(video) = local_stream
        .get_video_tracks()
        .get(0)
        .dyn_into::<MediaStreamTrack>()
    {
        if let Ok(video_stream) = MediaStream::new_with_tracks(&Array::of1(&video)) {
            info!("Host: set video");
            video_media_setter.call((self_id, video_stream));
        } else {
            warn!("Host: Cant make video");
        }
    }

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
                                }
                            });
                        }
                    }
                }
            }
        });
        create_effect(move |_| {
            if let Some((id, candidate)) = ice_signal.get() {
                info!("Host: received ice {id} {candidate}");

                if id == user {
                    if let Ok(candidate) = deserialize_candidate(&candidate) {
                        let _ =
                            pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate));
                    } else {
                        warn!("Cant deserialize candidate")
                    }
                }
            }
        });
    });

    Ok(())
}

pub fn receive_peer_connections(
    self_id: Callback<(), Option<Uuid>>,
    rtc_config: Callback<(), Option<RtcConfig>>,

    permissions_callback: Callback<Uuid, (bool, bool)>,

    video_media_setter: Callback<(Uuid, MediaStream), ()>,
    audio_media_setter: Callback<(Uuid, MediaStream), ()>,

    ice_callback: Callback<(Uuid, String)>,
    session_callback: Callback<(Uuid, RTCSessionDesc)>,

    ice_signal: Signal<Option<(Uuid, String)>>,
    session_signal: Signal<Option<(Uuid, RTCSessionDesc)>>,

    owner: Owner,
) {
    let peers = store_value(HashMap::<Uuid, RtcPeerConnection>::new());
    let pending_candidates = store_value(HashMap::<Uuid, Vec<RtcIceCandidateInit>>::new());

    create_effect(move |_| {
        if let Some((from_user, candidate)) = ice_signal.get() {
            if let Ok(candidate) = deserialize_candidate(&candidate) {
                if let Some(pc) = peers.with_value(|peers| peers.get(&from_user).cloned()) {
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
                        leptos::ev::Custom::<RtcTrackEvent>::new("track"),
                        move |ev| {
                            info!("Peer: ev: track");
                            let track = ev.track();
                            if let Ok(stream) = MediaStream::new_with_tracks(&Array::of1(&track)) {
                                if track.kind() == "audio" {
                                    audio_media_setter.call((from_user, stream));
                                } else {
                                    video_media_setter.call((from_user, stream));
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

async fn accept_peer_connection(
    self_id: Uuid,

    pc: &RtcPeerConnection,
    rtc_session_desc: RTCSessionDesc,

    video: bool,
    audio: bool,
    video_media_setter: Callback<(Uuid, MediaStream), ()>,
    audio_media_setter: Callback<(Uuid, MediaStream), ()>,
) -> Result<RTCSessionDesc, JsValue> {
    let local_stream = add_media_tracks(&pc, video, audio).await?;

    if let Ok(audio) = local_stream
        .get_audio_tracks()
        .get(0)
        .dyn_into::<MediaStreamTrack>()
    {
        if let Ok(audio_stream) = MediaStream::new_with_tracks(&Array::of1(&audio)) {
            audio_media_setter.call((self_id, audio_stream));
        }
    }
    if let Ok(video) = local_stream
        .get_video_tracks()
        .get(0)
        .dyn_into::<MediaStreamTrack>()
    {
        if let Ok(video_stream) = MediaStream::new_with_tracks(&Array::of1(&video)) {
            video_media_setter.call((self_id, video_stream));
        }
    }

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
