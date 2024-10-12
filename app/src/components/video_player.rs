use common::PlayerStatus;
use leptos::*;
use leptos_use::{
    use_event_listener, use_throttle_fn_with_arg, use_timeout_fn, UseTimeoutFnReturn,
};
use logging::warn;
use tracing::info;
use wasm_bindgen::JsCast;

use crate::{
    components::toaster::{Toast, Toaster},
    networking::room_manager::RoomManager,
    MountPoints,
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum VideoState {
    Playing,
    Paused,
    Waiting,
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
            VideoState::Waiting => write!(f, "Waiting"),
            VideoState::Ended => write!(f, "Ended"),
            VideoState::Errored => write!(f, "Errored"),
            VideoState::Stalled => write!(f, "Stalled"),
            VideoState::Suspend => write!(f, "Suspend"),
            VideoState::Seeking => write!(f, "Seeking"),
        }
    }
}

#[component]
pub fn VideoPlayer(#[prop(into)] src: Signal<Option<String>>) -> impl IntoView {
    let video_node = create_node_ref::<leptos::html::Video>();

    let (video_state, set_video_state) = create_signal(VideoState::Waiting);

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

    create_effect(move |_| {
        if let Some(video) = video_node.get() {
            if let Some(message) = player_messages_receiver.get() {
                let player_status = match video_state.get_untracked() {
                    VideoState::Playing => PlayerStatus::Playing(0.0),
                    VideoState::Paused
                    | VideoState::Waiting
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
                                if let Err(err) = video.play() {
                                    video.set_current_time(*time);
                                    warn!("Can not play video {err:#?}")
                                }
                            }
                        }
                        crate::networking::room_manager::PlayerMessages::Pause(time) => {
                            if !player_status.is_paused() {
                                info!("Received pause");
                                if let Err(err) = video.pause() {
                                    video.set_current_time(*time);
                                    warn!("Can not play video {err:#?}")
                                }
                            }
                        }
                        crate::networking::room_manager::PlayerMessages::Update(_) => {}
                        crate::networking::room_manager::PlayerMessages::Seek(time) => {
                            video.set_current_time(*time);
                        }
                    }

                    match message {
                        crate::networking::room_manager::PlayerMessages::Play(time)
                        | crate::networking::room_manager::PlayerMessages::Pause(time)
                        | crate::networking::room_manager::PlayerMessages::Update(time)
                        | crate::networking::room_manager::PlayerMessages::Seek(time) => {
                            if let Some(current_time) = current_time.get_untracked() {
                                if ((current_time - time) as f64).abs() > 15.0 {
                                    info!("Time difference big, seeking to time");
                                    video.set_current_time(time);
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
        let player_status = match video_state {
            VideoState::Playing => PlayerStatus::Playing(time),
            VideoState::Paused
            | VideoState::Waiting
            | VideoState::Ended
            | VideoState::Errored
            | VideoState::Stalled
            | VideoState::Suspend
            | VideoState::Seeking => PlayerStatus::Paused(time),
        };
        if video_state != VideoState::Seeking {
            if let Some(room_player_status) = room_manager.get_player_status() {
                if room_player_status.is_paused() != player_status.is_paused() {
                    room_manager.set_player_status(player_status.clone());
                    match player_status {
                        PlayerStatus::Paused(time) => {
                            info!("Sending pause");
                            room_manager.send_message(
                                common::message::ClientMessage::Pause(time),
                                crate::networking::room_manager::SendType::Reliable,
                            );
                        }
                        PlayerStatus::Playing(time) => {
                            info!("Sending play");
                            room_manager.send_message(
                                common::message::ClientMessage::Play(time),
                                crate::networking::room_manager::SendType::Reliable,
                            );
                        }
                    }
                }
            }
        }
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
            send_update_throttled(time);
        }
    });

    let video_base_ref = create_node_ref::<leptos::html::Div>();

    let mount_points = expect_context::<MountPoints>();

    mount_points.full_screen_point.set(Some(video_base_ref));

    create_effect(move |_| {
        info!("Register fullscreenchange");
        let _ = use_event_listener(document(), leptos::ev::fullscreenchange, move |_| {
            info!("Fullschreen changed");
            set_is_full_screen.set(document().fullscreen_element().is_some());
        });
    });

    let (chat_msg, set_chat_msg) = create_signal(String::new());

    view! {
        <div
            ref=video_base_ref
            class="h-full w-full flex flex-col"
            class=("hidden", move || src.with(|v| v.is_none()))
        >
            <div class="flex-1 overflow-auto w-full relative">
                <video
                    ref=video_node
                    class="h-full w-full"
                    on:canplay=move |_| {
                        if let Some(video) = video_node.get_untracked() {
                            if video.paused() {
                                set_video_state.set(VideoState::Paused);
                            } else {
                                set_video_state.set(VideoState::Playing);
                            }
                        }
                    }
                    on:canplaythrough=move |_| {
                        if let Some(video) = video_node.get_untracked() {
                            if video.paused() {
                                set_video_state.set(VideoState::Paused);
                            } else {
                                set_video_state.set(VideoState::Playing);
                            }
                        }
                    }
                    on:ended=move |_| {
                        set_video_state.set(VideoState::Ended);
                    }
                    on:error=move |_| { set_video_state.set(VideoState::Errored) }
                    on:pause=move |_| { set_video_state.set(VideoState::Paused) }
                    on:play=move |_| { set_video_state.set(VideoState::Playing) }
                    on:playing=move |_| { set_video_state.set(VideoState::Playing) }
                    on:stalled=move |_| { set_video_state.set(VideoState::Stalled) }
                    on:suspend=move |_| { set_video_state.set(VideoState::Suspend) }
                    on:waiting=move |_| { set_video_state.set(VideoState::Waiting) }
                    on:seeking=move |_| { set_video_state.set(VideoState::Seeking) }
                    on:seeked=move |_| {
                        if let Some(video) = video_node.get() {
                            if video.paused() {
                                set_video_state.set(VideoState::Paused)
                            } else {
                                set_video_state.set(VideoState::Playing)
                            }
                        }
                    }

                    on:durationchange=move |_| {
                        if let Some(video) = video_node.get() {
                            set_duration.set(Some(video.duration()));
                        }
                    }
                    on:timeupdate=move |_| {
                        if let Some(video) = video_node.get() {
                            set_current_time.set(Some(video.current_time()));
                        }
                    }
                >
                    {move || {
                        if let Some(url) = src.get() {
                            view! { <source src=url /> }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </video>

                {if let Some((message_signal, message_history)) = expect_context::<RoomManager>()
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
                            class=("hidden", move || !(is_full_screen.get() && is_visible.get()))
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
                            match video_state.get_untracked() {
                                VideoState::Playing => {
                                    if let Some(video) = video_node.get_untracked() {
                                        if let Err(err) = video.pause() {
                                            warn!("Errored Playing {err:#?}");
                                        }
                                    }
                                }
                                VideoState::Paused | VideoState::Waiting => {
                                    if let Some(video) = video_node.get_untracked() {
                                        if let Err(err) = video.play() {
                                            warn!("Errored Pausing {err:#?}");
                                        }
                                    }
                                }
                                state => info!("Cant do anything in state {state}"),
                            }
                        }
                    >
                        {move || match video_state.get() {
                            VideoState::Playing => "Pause".to_string(),
                            VideoState::Paused => "Play".to_string(),
                            VideoState::Waiting => {
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
                                    if VideoState::Seeking != video_state.get_untracked() {
                                        let new_time = (x as f64) / (width as f64) * total;
                                        video.set_current_time(new_time);
                                        room_manager_c
                                            .send_message(
                                                common::message::ClientMessage::Seek(new_time),
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

                    </div>

                    <div class="absolute top-[85%] left-[5%]">
                        <button on:click=move |_| {
                            if let Some(video_base) = video_base_ref.get_untracked() {
                                let toaster = expect_context::<Toaster>();
                                if !is_full_screen.get_untracked() {
                                    if let Err(err) = video_base.request_fullscreen() {
                                        warn!("Cannot enter full screen {err:?}");
                                        toaster.toast(Toast{
                                            message: format!("Full screen failed {err:?}").into(),
                                            r#type: crate::components::toaster::ToastType::Failed,
                                        });
                                    } else if let Ok(screen) = window().screen() {
                                        if let Err(err) = screen
                                            .orientation()
                                            .lock(web_sys::OrientationLockType::Landscape)
                                        {
                                            warn!("Cant lock orientation {err:?}")
                                        }
                                    }
                                } else {
                                    document().exit_fullscreen();
                                    if let Ok(screen) = window().screen() {
                                        if let Err(err) = screen.orientation().unlock() {
                                            warn!("Cant unlock orientation {err:?}")
                                        }
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
