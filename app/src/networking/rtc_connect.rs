use tracing::info;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    js_sys::{Array, JSON},
    window, MediaStream, MediaStreamConstraints, RtcConfiguration, RtcIceCandidate,
    RtcIceCandidateInit, RtcIceServer, RtcPeerConnection, RtcSessionDescriptionInit,
};

pub fn connect_rtc(
    stun: &str,
    turn: &str,
    turn_user: &str,
    turn_pass: &str,
) -> Result<RtcPeerConnection, JsValue> {
    RtcPeerConnection::new_with_configuration(&{
        let config = RtcConfiguration::new();
        config.set_ice_servers(&{
            let array = Array::new();
            array.push(&JsValue::from({
                let ice_server = RtcIceServer::new();
                ice_server.set_urls(&JsValue::from_str(stun));
                ice_server
            }));
            array.push(&JsValue::from({
                let ice_server = RtcIceServer::new();
                ice_server.set_urls(&JsValue::from_str(turn));
                ice_server.set_username(turn_user);
                ice_server.set_credential(turn_pass);
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

pub async fn add_media_tracks(pc: &RtcPeerConnection) -> Result<MediaStream, JsValue> {
    let user_media = window()
        .unwrap()
        .navigator()
        .media_devices()
        .expect("No Media Devices")
        .get_user_media_with_constraints(&{
            let constraints = MediaStreamConstraints::new();
            constraints.set_audio(&JsValue::from_bool(true));
            constraints.set_video(&JsValue::from_bool(true));
            constraints
        })?;
    let media_stream = wasm_bindgen_futures::JsFuture::from(user_media)
        .await?
        .dyn_into::<MediaStream>()?;

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
