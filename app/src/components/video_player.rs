use core::time;

use common::PlayerStatus;
use leptos::*;
use leptos_use::{
    use_interval_fn, use_throttle_fn, use_throttle_fn_with_arg, use_timeout_fn, UseTimeoutFnReturn,
};
use logging::warn;
use tracing::info;

use crate::networking::room_manager::RoomManager;

#[derive(Debug, Clone, Copy)]
enum VideoState {
    Playing,
    Paused,
    Waiting,
    Ended,
    Errored,
    Stalled,
    Suspend,
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

    let player_messages_receiver = room_manager.get_player_messages();
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
                    | VideoState::Suspend => PlayerStatus::Paused(0.0),
                };
                match &message {
                    crate::networking::room_manager::PlayerMessages::Play(time) => {
                        if player_status.is_paused() {
                            video.set_current_time(*time);
                            if let Err(err) = video.play() {
                                warn!("Can not play video {err:#?}")
                            }
                        }
                    }
                    crate::networking::room_manager::PlayerMessages::Pause(time) => {
                        if !player_status.is_paused() {
                            video.set_current_time(*time);
                            if let Err(err) = video.pause() {
                                warn!("Can not play video {err:#?}")
                            }
                        }
                    }
                    crate::networking::room_manager::PlayerMessages::Update(_) => {}
                    crate::networking::room_manager::PlayerMessages::Seek(_) => {}
                }
                match message {
                    crate::networking::room_manager::PlayerMessages::Play(time)
                    | crate::networking::room_manager::PlayerMessages::Pause(time)
                    | crate::networking::room_manager::PlayerMessages::Update(time)
                    | crate::networking::room_manager::PlayerMessages::Seek(time) => {
                        if let Some(current_time) = current_time.get_untracked() {
                            if ((current_time as f64) - time).abs() > 15.0 {
                                info!("Time difference big, seeking to time");
                                video.set_current_time(time);
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
            | VideoState::Suspend => PlayerStatus::Paused(time),
        };
        if let Some(room_player_status) = room_manager.get_player_status() {
            if room_player_status.is_paused() != player_status.is_paused() {
                room_manager.set_player_status(player_status.clone());
                match player_status {
                    PlayerStatus::Paused(time) => {
                        room_manager.send_message(
                            common::message::ClientMessage::Pause(time),
                            crate::networking::room_manager::SendType::Reliable,
                        );
                    }
                    PlayerStatus::Playing(time) => {
                        room_manager.send_message(
                            common::message::ClientMessage::Play(time),
                            crate::networking::room_manager::SendType::Reliable,
                        );
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

    view! {
        <div class="h-full w-full relative" class=("hidden", move || src.with(|v| v.is_none()))>
            <video
                ref=video_node
                class="h-full w-full"
                on:canplay=move |_| {
                    set_video_state.set(VideoState::Paused);
                }
                on:canplaythrough=move |_| {
                    set_video_state.set(VideoState::Paused);
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
            <div
                class="absolute h-full w-full top-0 left-0 bg-black/70 opacity-0 hover:opacity-100
                flex flex-col items-center justify-center
                "
                class=("opacity-100", is_ui_open)
                on:touchend=move |_| {
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
                        VideoState::Waiting => "Play".to_string(),
                        state => state.to_string(),
                    }}
                </button>

                {move || {
                    if let (Some(elapsed), Some(total)) = (current_time.get(), duration.get()) {
                        view! {
                            <div class="absolute w-[90%] top-[80%] left-[5%] h-4 bg-white/45">
                                <div
                                    class="absolute top-0 left-0 h-full bg-white"
                                    style=format!("width: {}%;", elapsed * 100.0 / total)
                                />
                            </div>
                        }
                            .into_view()
                    } else {
                        view! {}.into_view()
                    }
                }}
            </div>
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
