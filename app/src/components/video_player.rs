use std::{future::Future, pin::Pin};

use common::PlayerStatus;
use futures::FutureExt;
use leptos::*;
use leptos_use::{
    use_event_listener, use_interval_fn, use_throttle_fn_with_arg, use_timeout_fn,
    UseIntervalReturn, UseTimeoutFnReturn,
};
use logging::warn;
use tracing::{debug, info};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::{Element, MediaStream};

use crate::{
    components::toaster::{Toast, ToastType, Toaster},
    networking::room_manager::RoomManager,
    tauri_provider::FullScreenProvider,
    utils::download_logs,
    LogProvider, MountPoints,
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum VideoState {
    Playing,
    Paused,
    Loading,
    Ended,
    Errored,
    Stalled,
    Suspend,
    Seeking,
}

impl std::fmt::Display for VideoState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            VideoState::Playing => write!(f, "Playing"),
            VideoState::Paused => write!(f, "Paused"),
            VideoState::Loading => write!(f, "Waiting"),
            VideoState::Ended => write!(f, "Ended"),
            VideoState::Errored => write!(f, "Errored"),
            VideoState::Stalled => write!(f, "Stalled"),
            VideoState::Suspend => write!(f, "Suspend"),
            VideoState::Seeking => write!(f, "Seeking"),
        }
    }
}

#[derive(Clone)]
pub enum VideoSource {
    Url(String),
    Stream((Uuid, MediaStream)),
}

#[derive(Clone, PartialEq)]
pub enum VideoType {
    None,
    Local,
    LocalStreamingOut,
    RemoteStreamingIn,
}

#[component]
pub fn VideoPlayer(#[prop(into)] src: Signal<Option<VideoSource>>) -> impl IntoView {
    let video_node = create_node_ref::<leptos::html::Video>();

    let (video_state, set_video_state) = create_signal(VideoState::Loading);

    let (current_time, set_current_time) = create_signal(None);
    let (duration, set_duration) = create_signal(None);

    let (is_ui_open, set_is_ui_open) = create_signal(false);
    let UseTimeoutFnReturn {
        start: start_close_timeout,
        stop: stop_close_tiemout,
        ..
    } = use_timeout_fn(
        move |_| {
            set_is_ui_open.set(false);
        },
        3000.0,
    );

    let room_manager = expect_context::<RoomManager>();
    let room_manager_c = room_manager.clone();

    let player_messages_receiver = room_manager.get_player_messages();

    let (is_full_screen, set_is_full_screen) = create_signal(false);

    let share_permission_sig = room_manager.share_video_permission;

    let video_type = store_value(VideoType::None);

    let before_seek = store_value(Option::<bool>::None);
    create_effect({
        let room_manager = room_manager.clone();

        move |_| {
            if let Some(share_user) = share_permission_sig.get() {
                let room_manager = room_manager.clone();
                leptos::spawn_local(async move {
                    if let Err(err) = room_manager.add_video_share(share_user, video_node).await {
                        warn!("Add video share error {err:?}");
                    } else {
                        video_type.set_value(VideoType::LocalStreamingOut);
                    }
                });
            }
        }
    });

    let owner = Owner::current();
    create_effect(move |_| {
        if let Some(video_source_type) = src.get() {
            match video_source_type {
                VideoSource::Url(_) => {
                    video_type.set_value(VideoType::Local);
                    let UseTimeoutFnReturn { start, .. } =
                        with_owner(owner.expect("Player owner expected"), || {
                            use_timeout_fn(
                                move |_: ()| {
                                    if let Some(video) = video_node.get_untracked() {
                                        video.load()
                                    }
                                },
                                100.0,
                            )
                        });
                    start(());
                }
                VideoSource::Stream(_) => {
                    video_type.set_value(VideoType::RemoteStreamingIn);
                }
            }
        }
    });

    let (time_range, set_time_range) = create_signal(None);

    use_interval_fn(
        move || {
            if let Some(video) = video_node.get_untracked() {
                set_time_range.set(Some(video.buffered()));
            }
        },
        1000,
    );

    create_effect(move |_| {
        if let Some(video) = video_node.get() {
            if let Some(message) = player_messages_receiver.get() {
                let player_status = match video_state.get_untracked() {
                    VideoState::Playing => PlayerStatus::Playing(0.0),
                    VideoState::Paused
                    | VideoState::Loading
                    | VideoState::Ended
                    | VideoState::Errored
                    | VideoState::Stalled
                    | VideoState::Suspend
                    | VideoState::Seeking => PlayerStatus::Paused(0.0),
                };

                if video_state.get_untracked() != VideoState::Seeking {
                    match &message {
                        crate::networking::room_manager::PlayerMessages::Play(time) => {
                            if player_status.is_paused() {
                                info!("Received play");
                                info!("Set current time on play {time}");
                                video.set_current_time(*time);
                                if let Err(err) = video.play() {
                                    warn!("Can not play video {err:#?}")
                                }
                            }
                        }
                        crate::networking::room_manager::PlayerMessages::Pause(time) => {
                            if !player_status.is_paused() {
                                info!("Received pause");

                                info!("Set current time on pause {time}");
                                video.set_current_time(*time);
                                if let Err(err) = video.pause() {
                                    warn!("Can not play video {err:#?}")
                                }
                            }
                        }
                        crate::networking::room_manager::PlayerMessages::Update(_) => {}
                        crate::networking::room_manager::PlayerMessages::Seek(time, beforeseek) => {
                            match video_type.get_value() {
                                VideoType::None
                                | VideoType::Local
                                | VideoType::LocalStreamingOut => {
                                    video.pause();

                                    info!("Set current time on seek {time}");
                                    video.set_current_time(*time);
                                    before_seek.set_value(Some(*beforeseek));
                                }
                                VideoType::RemoteStreamingIn => {
                                    //Ignore
                                }
                            }
                        }
                    }

                    if video_state.get_untracked() == VideoState::Paused
                        || video_state.get_untracked() == VideoState::Playing
                    {
                        match message {
                            crate::networking::room_manager::PlayerMessages::Play(time)
                            | crate::networking::room_manager::PlayerMessages::Pause(time)
                            | crate::networking::room_manager::PlayerMessages::Update(time)
                            | crate::networking::room_manager::PlayerMessages::Seek(time, _) => {
                                if let Some(current_time) = current_time.get_untracked() {
                                    if ((current_time - time) as f64).abs() > 15.0 {
                                        info!("Time difference big, seeking to time");
                                        match video_type.get_value() {
                                            VideoType::None
                                            | VideoType::Local
                                            | VideoType::LocalStreamingOut => {
                                                info!("Set current time on difference {time}");
                                                video.set_current_time(time);
                                            }
                                            VideoType::RemoteStreamingIn => {
                                                //Ignore
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    create_effect(move |_| {
        let video_state = video_state.get();
        let time = current_time.get_untracked().unwrap_or_default();

        match video_state {
            VideoState::Playing => room_manager.send_message(
                common::message::ClientMessage::Play(time),
                crate::networking::room_manager::SendType::Reliable,
            ),
            VideoState::Paused => room_manager.send_message(
                common::message::ClientMessage::Pause(time),
                crate::networking::room_manager::SendType::Reliable,
            ),
            _ => {}
        };
    });

    let send_update_throttled = use_throttle_fn_with_arg(
        |time| {
            let room_manager = expect_context::<RoomManager>();

            room_manager.send_message(
                common::message::ClientMessage::Update(time),
                crate::networking::room_manager::SendType::UnReliablle,
            );
        },
        3000.0,
    );

    create_effect(move |_| {
        if let Some(time) = current_time.get() {
            if video_type.get_value() != VideoType::RemoteStreamingIn
                && video_state.get() == VideoState::Paused
                || video_state.get() == VideoState::Playing
            {
                send_update_throttled(time);
            }
        }
    });

    let mount_points = expect_context::<MountPoints>();

    let main_ref = mount_points.main_point;
    create_effect(move |_| {
        info!("Register fullscreenchange");
        let _ = use_event_listener(document(), leptos::ev::fullscreenchange, move |_| {
            info!("Fullschreen changed");
            set_is_full_screen.set(document().fullscreen_element().is_some());
        });
    });

    let (chat_msg, set_chat_msg) = create_signal(String::new());

    let room_info = expect_context::<RoomManager>().get_room_info();

    let (is_seeking, set_is_seeking) = create_signal(false);

    let (retry, set_retry) = create_signal(0);
    create_effect(move |_| {
        if let Some(VideoSource::Stream((user_id, stream))) = src.get() {
            if let Some(video_node) = video_node.get_untracked() {
                video_node.set_src_object(Some(&stream));
                let video_length = create_memo(move |_| {
                    let user = room_info
                        .with_untracked(|r| {
                            r.as_ref()
                                .map(|r| r.users.iter().find(|user| user.id == user_id).cloned())
                        })
                        .flatten();
                    if let Some(user) = user {
                        user.state.as_video_selected().and_then(|v| v.duration)
                    } else {
                        None
                    }
                });
                create_effect(move |_| {
                    set_duration.set(video_length.get());
                });
            }
        }
    });

    create_effect(move |_| {
        if let Some(player_status) = room_info.with(|r| r.as_ref().map(|r| r.player_status.clone()))
        {
            info!("Player status {player_status:?}");
            if video_type.get_value() == VideoType::RemoteStreamingIn {
                set_current_time.set(Some(player_status.get_time()));
            }
        }
    });

    view! {
        <div
            class="h-full w-full flex flex-col"
            class=("hidden", move || src.with(|v| v.is_none()))
        >
            <div class="flex-1 overflow-auto w-full relative">
                <video
                    playsinline=true
                    disableRemotePlayback=true
                    ref=video_node
                    class="h-full w-full"
                    preload="auto"
                    on:canplay=move |_| {
                        debug!("video: Received canplay");
                        if let Some(video) = video_node.get_untracked() {
                            if video.paused() {
                                set_video_state.set(VideoState::Paused);
                            } else {
                                set_video_state.set(VideoState::Playing);
                            }
                        }
                    }
                    on:canplaythrough=move |_| {
                        debug!("video: Received canplaythrough");
                        if let Some(video) = video_node.get_untracked() {
                            if video.paused() {
                                set_video_state.set(VideoState::Paused);
                            } else {
                                set_video_state.set(VideoState::Playing);
                            }
                        }
                    }
                    on:ended=move |_| {
                        debug!("video: Received ended");
                        set_video_state.set(VideoState::Ended);
                    }
                    on:error=move |_| {
                        let toaster = expect_context::<Toaster>();
                        if retry.get_untracked() < 4 {
                            toaster.toast(Toast{
                                message:"Video Errored, retrying".into(),
                                r#type: ToastType::Failed,
                            });
                            set_retry.set(retry.get_untracked() + 1);
                        }else{
                            toaster.toast(Toast{
                                message:"Video Errored many time, giving up".into(),
                                r#type: ToastType::Failed,
                            });
                        }
                        debug!("video: Received error");
                        set_video_state.set(VideoState::Errored);
                    }
                    on:pause=move |_| {
                        debug!("video: Received pause");
                        set_video_state.set(VideoState::Paused);
                    }
                    on:play=move |_| {
                        debug!("video: Received play");
                        set_video_state.set(VideoState::Playing);
                        set_is_seeking.set(false);
                    }
                    on:playing=move |_| {
                        debug!("video: Received playing");
                        set_video_state.set(VideoState::Playing) }
                    on:stalled=move |_| {
                        debug!("video: Received stalled");
                        set_video_state.set(VideoState::Stalled) }
                    on:suspend=move |_| {
                        debug!("video: Received suspend");
                        set_video_state.set(VideoState::Suspend) }
                    on:waiting=move |_| {
                        debug!("video: Received waiting");
                        set_video_state.set(VideoState::Loading);
                    }
                    on:seeking=move |_| {
                        debug!("video: Received seeking");
                        set_video_state.set(VideoState::Seeking) }
                    on:seeked=move |_| {
                        debug!("video: Received seeked");
                        if video_type.get_value() != VideoType::RemoteStreamingIn {
                            if let Some(video) = video_node.get() {
                                if video.paused() {
                                    set_video_state.set(VideoState::Paused)
                                } else {
                                    set_video_state.set(VideoState::Playing)
                                }

                                if let Some(beforeseek) = before_seek.get_value(){
                                    if beforeseek {
                                        video.play();
                                    }else{
                                        video.pause();
                                    }
                                    before_seek.set_value(None);
                                }
                            }
                        }
                    }

                    on:durationchange=move |_| {
                        debug!("video: Received durationchange");
                        if video_type.get_value() != VideoType::RemoteStreamingIn {
                            if let Some(video) = video_node.get() {
                                let rm = expect_context::<RoomManager>();
                                set_duration.set(Some(video.duration()));
                                rm.set_video_duration(video.duration());
                            }
                        }
                    }
                    on:timeupdate=move |_| {
                        debug!("video: Received timeupdate");
                        if let Some(video) = video_node.get() {
                            if video_type.get_value() != VideoType::RemoteStreamingIn {
                                set_current_time.set(Some(video.current_time()));
                            }
                        }
                    }
                >
                    {move || {
                        retry.get();
                        if let Some(VideoSource::Url(url)) = src.get() {
                            view! { <source src=url /> }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </video>

                {
                    let r_i = expect_context::<RoomManager>().get_room_info();
                    let is_connected = create_memo(move|_|r_i.with(|r|r.is_some()));
                move || if is_connected.get() {
                    if let Some((message_signal, message_history)) = expect_context::<RoomManager>()
                    .get_chat_signal()
                    {
                        let (msg_len, set_msg_len) = create_signal(
                            message_history.with_value(|v| v.len()),
                        );
                        let (is_visible, set_is_visible) = create_signal(false);
                        let UseTimeoutFnReturn {
                            start: start_visible_timeout,
                            stop: stop_visible_tiemout,
                            ..
                        } = use_timeout_fn(
                            move |_| {
                                set_is_visible.set(false);
                            },
                            5000.0,
                        );
                        create_effect(move |_| {
                            message_signal.with(|_| ());
                            set_msg_len.set(message_history.with_value(|v| v.len()));
                            set_is_visible.set(true);
                            stop_visible_tiemout();
                            start_visible_timeout(());
                        });
                        view! {
                            <div
                                class="absolute w-[20%] overflow-auto break-words right-0 top-[35%] h-[30%] flex flex-col-reverse"
                                class=("hidden", move || !( is_visible.get()))
                            >
                                <For
                                    each=move || {
                                        let len = msg_len.get();
                                        (0..len).rev()
                                    }
                                    key=|i| *i
                                    children=move |i| {
                                        let msg = message_history.with_value(|v| v.get(i).cloned());
                                        if let Some((user, msg)) = msg {
                                            view! {
                                                <div class="w-full text-md font-thin14 [text-shadow:_0_1px_0_rgb(0_0_0_/_40%)]">
                                                    <span class="font-thin8 text-sm">{user.name} ": "</span>
                                                    <span>{msg}</span>
                                                </div>
                                            }
                                                .into_view()
                                        } else {
                                            view! {}.into_view()
                                        }
                                    }
                                />
                            </div>
                        }
                            .into_view()
                    } else {
                        view! {}.into_view()
                    }
                }else{
                    view! {}.into_view()
                }}
                <div
                    class="absolute h-full w-full top-0 left-0 bg-black/70 opacity-0
                    flex flex-col items-center justify-center
                    "
                    class=("opacity-100", is_ui_open)
                    on:mousemove=move |_| {
                        set_is_ui_open.set(true);
                        stop_close_tiemout();
                        start_close_timeout(());
                    }
                >
                    <div class="absolute w-full top-0 left-0 p-8 flex text-sm">
                        <div>{move || format_time(current_time.get())}</div>
                        <div class="flex-grow" />
                        <div>{move || format_time(duration.get())}</div>
                    </div>
                    <button
                        type="button"
                        class="text-2xl font-bold2"
                        on:click=move |_| {
                            let toaster = expect_context::<Toaster>();
                            match video_state.get_untracked() {
                                VideoState::Playing => {
                                    if let Some(video) = video_node.get_untracked() {
                                        if let Err(err) = video.pause() {
                                            toaster.toast(
                                                Toast { message: format!("Errored Pausing {err:#?}").into(), r#type: crate::components::toaster::ToastType::Failed }
                                            );
                                        }
                                    }
                                }
                                _ => {
                                    if let Some(video) = video_node.get_untracked() {
                                        if let Err(err) = video.play() {
                                            toaster.toast(
                                                Toast { message: format!("Errored Playing {err:#?}").into(), r#type: crate::components::toaster::ToastType::Failed }
                                            );
                                        }
                                    }
                                }

                            }
                        }
                    >
                        {move || match video_state.get() {
                            VideoState::Playing => "Pause".to_string(),
                            VideoState::Paused => "Play".to_string(),
                            VideoState::Loading => {
                                if let Some(video) = video_node.get() {
                                    if video.current_time() == 0.0 { "Play" } else { "Waiting" }
                                } else {
                                    "Waiting"
                                }
                                    .to_string()
                            }
                            state => state.to_string(),
                        }}
                    </button>
                    {move || {
                        if let Some(VideoSource::Url(url)) = src.get() {
                            view! {
                                <button
                                    class="text-2xl font-bold2"
                                    on:click=move|_|{
                                        if let Some(log_prov) = use_context::<LogProvider>(){
                                            download_logs(log_prov.logs.get_value());
                                        }
                                    }
                                >
                                    "Download Logs"
                                </button>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}

                    <div
                        class="absolute w-[90%] top-[80%] left-[5%] h-4 bg-white/45 cursor-pointer"
                        on:click=move |ev| {
                            let x = ev.offset_x();
                            if let Some(element) = ev.target() {
                                let width = element
                                    .unchecked_into::<web_sys::HtmlElement>()
                                    .offset_width();
                                if let (Some(video), Some(total)) = (
                                    video_node.get_untracked(),
                                    duration.get_untracked(),
                                ) {
                                    let new_time = (x as f64) / (width as f64) * total;
                                    if video_type.get_value() == VideoType::RemoteStreamingIn {
                                        room_manager_c
                                        .send_message(
                                            common::message::ClientMessage::Seek(new_time, !video.paused()),
                                            crate::networking::room_manager::SendType::Reliable,
                                        );
                                    }else if VideoState::Seeking != video_state.get_untracked() {
                                        let is_playing = !video.paused();
                                        video.pause();

                                        info!("Set current time on seek local {new_time}");
                                        video.set_current_time(new_time);
                                        set_is_seeking.set(true);
                                        room_manager_c
                                            .send_message(
                                                common::message::ClientMessage::Seek(new_time, is_playing),
                                                crate::networking::room_manager::SendType::Reliable,
                                            );
                                    }
                                }
                            }
                        }
                    >
                        <div
                            class="absolute top-0 left-0 h-full bg-white pointer-events-none"
                            style=move || {
                                if let (Some(elapsed), Some(total)) = (
                                    current_time.get(),
                                    duration.get(),
                                ) {
                                    format!("width: {}%;", elapsed * 100.0 / total)
                                } else {
                                    "".to_string()
                                }
                            }
                        />

                        {
                            move || {
                                let timeranges = time_range.get();
                                if let (Some(timeranges), Some(total)) = (timeranges, duration.get()) {

                                    let mut views = vec![];

                                    for i in 0..timeranges.length() {
                                        let start = timeranges.start(i);
                                        let end = timeranges.end(i);
                                        if let (Ok(start), Ok(end)) = (start, end) {
                                            let style = format!("left: {}%; width: {}%;", start * 100.0 / total, (end-start) *100.0 / total );
                                            views.push(view! {
                                                <div
                                                    class="absolute top-0 h-full bg-red-500/50 pointer-events-none"
                                                    style=style
                                                />
                                            });
                                        }
                                    }
                                    views.into_view()
                                }else{
                                    view! {}.into_view()
                                }
                            }
                        }

                    </div>

                    <div class="absolute top-[85%] left-[5%]">
                        <button on:click=move |_| {
                            if let Some(video_base) = main_ref.get_untracked() {
                                let fullscreenprovider = use_context::<FullScreenProvider>();
                                if !is_full_screen.get_untracked() {
                                    if let Some(fullscreenprovider) = fullscreenprovider {
                                        let el: &Element = video_base.as_ref();
                                        fullscreenprovider.fullscreen.call(el.clone());
                                    }
                                } else {
                                    if let Some(fullscreenprovider) = fullscreenprovider {
                                        fullscreenprovider.exit_fullscreen.call(());
                                    }
                                }
                            }
                        }>"[ Full Screen ]"</button>
                    </div>
                </div>

            </div>

            <form
                class="w-full flex"
                class=("hidden", move || !is_full_screen.get())
                on:submit=move |ev| {
                    ev.prevent_default();
                    let rm = expect_context::<RoomManager>();
                    rm.send_chat(chat_msg.get_untracked());
                    set_chat_msg.set(String::new());
                }
            >
                <input
                    class="w-full text-md font-thin16 p-2 bg-transparent text-white"
                    placeholder="Enter msg to chat"
                    on:input=move |ev| { set_chat_msg.set(event_target_value(&ev)) }
                    on:keyup=move |ev| {
                        if ev.key_code() == 13 || ev.key() == "Enter" {
                            let rm = expect_context::<RoomManager>();
                            rm.send_chat(chat_msg.get_untracked());
                            set_chat_msg.set(String::new());
                        }
                    }
                    prop:value=chat_msg
                />
                <button class="p-2 border text-md font-thin14 ">"Submit"</button>
            </form>
        </div>
    }
}

fn format_time(time: Option<f64>) -> String {
    if let Some(time) = time {
        let hours = (time / 3600.0).floor() as u32;
        let minutes = ((time % 3600.0) / 60.0).floor() as u32;
        let seconds = (time % 60.0).floor() as u32;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        "--:--:--".to_string()
    }
}
