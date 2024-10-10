use std::collections::HashMap;

use common::UserMeta;
use ev::PointerEvent;
use leptos::*;
use leptos_use::{use_window_size, UseWindowSizeReturn};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    components::{
        dialog::Dialog,
        icons::Icon,
        toaster::{Toast, Toaster},
    },
    networking::room_manager::RoomManager,
};

#[derive(Clone)]
struct VideoUser {
    user_meta: RwSignal<UserMeta>,
    video_ref: NodeRef<leptos::html::Video>,
    is_video_active: RwSignal<bool>,
}

impl PartialEq for VideoUser {
    fn eq(&self, other: &Self) -> bool {
        self.user_meta == other.user_meta && self.is_video_active == other.is_video_active
    }
}

#[component]
pub fn VideoChat() -> impl IntoView {
    let rm = expect_context::<RoomManager>();

    let (video_users, set_video_users) = create_signal(HashMap::<Uuid, VideoUser>::new());

    let room_info = rm.get_room_info();
    let owner = Owner::current().unwrap();

    create_effect(move |_| {
        if let Some(room_info) = room_info.get() {
            let vu = video_users.get_untracked();
            let mut new_users = HashMap::new();
            for user in room_info.users {
                if let Some(user_v) = vu.get(&user.id) {
                    let user_id = user.id;
                    user_v.user_meta.set(user);
                    new_users.insert(user_id, user_v.clone());
                } else {
                    new_users.insert(
                        user.id,
                        VideoUser {
                            user_meta: with_owner(owner, || create_rw_signal(user)),
                            video_ref: with_owner(owner, || create_node_ref()),
                            is_video_active: with_owner(owner, || create_rw_signal(false)),
                        },
                    );
                }
            }
            set_video_users.set(new_users);
        }
    });

    let video_receiver = rm.video_chat_stream_signal.0;

    create_effect(move |_| {
        if let Some((user_id, stream)) = video_receiver.get() {
            if let Some(VideoUser {
                video_ref,
                is_video_active,
                ..
            }) = video_users.with(|map| map.get(&user_id).cloned())
            {
                if let Some(video) = video_ref.get_untracked() {
                    info!("Playing video");
                    video.set_src_object(Some(&stream));
                    if let Err(err) = video.play() {
                        warn!("Cannot play audio {err:?}")
                    }
                    is_video_active.set(true);
                } else {
                    info!("No video in ref");
                }
            }
        }
    });

    let (position, set_position) = create_signal((10.0, 10.0));
    let (width, set_width) = create_signal(100.0);

    let diff = store_value(None);
    let pointer_events = store_value(HashMap::new());

    let pointer_up = move |ev: PointerEvent| {
        pointer_events.update_value(|p| {
            p.remove(&ev.pointer_id());
        });
        if pointer_events.with_value(|p| p.len()) < 2 {
            diff.update_value(|d| *d = None);
        }
    };

    let pointer_down = move |ev: PointerEvent| {
        pointer_events.update_value(|p| {
            p.insert(ev.pointer_id(), ev);
        });
    };

    let div_ref = create_node_ref::<leptos::html::Div>();

    let UseWindowSizeReturn {
        width: window_width,
        height: window_height,
    } = use_window_size();

    let pointer_move = move |ev: PointerEvent| {
        if let Some(div) = div_ref.get_untracked() {
            let (current_width, current_height) =
                (div.offset_width() as f32, div.offset_height() as f32);
            let (window_width, window_height) =
                (window_width.get_untracked(), window_height.get_untracked());

            pointer_events.update_value(|p| {
                if let Some(val) = p.get_mut(&ev.pointer_id()) {
                    *val = ev;
                }
            });
            if let (Some(ev1), Some(ev2)) = pointer_events.with_value(|p| {
                let mut it = p.values();
                (it.next().cloned(), it.next().cloned())
            }) {
                let current_diff = (((ev1.client_x() - ev2.client_x()) as f64).powi(2)
                    + ((ev1.client_y() - ev2.client_y()) as f64).powi(2))
                .sqrt();
                if let Some(diff) = diff.with_value(|d| *d) {
                    let max_width =
                        window_width.min(((current_width / current_height) as f64) * window_height);
                    let expected_new_width: f64 = width.get_untracked() + (current_diff - diff);
                    set_width.set(expected_new_width.min(max_width));
                    let new_width = width.get_untracked();
                    let new_height = ((current_height / current_width) as f64) * new_width;
                    let max_left = window_width - new_width;
                    let max_top = window_height - new_height;
                    let (x, y): (f32, f32) = position.get_untracked();
                    set_position.set((x.clamp(0.0, max_left as f32), y.clamp(0.0, max_top as f32)));
                }
                diff.update_value(|d| *d = Some(current_diff));
            } else if let Some(ev) = pointer_events.with_value(|p| {
                let mut it = p.values();
                it.next().cloned()
            }) {
                let (x, y) = position.get_untracked();
                set_position.set((
                    (x + ev.movement_x() as f32).clamp(0.0, (window_width as f32) - current_width),
                    (y + ev.movement_y() as f32)
                        .clamp(0.0, (window_height as f32) - current_height),
                ));
            }
        }
    };
    create_effect(move |_| {
        let (window_width, window_height) = (window_width.get(), window_height.get());
        if let Some(div) = div_ref.get_untracked() {
            let (current_width, current_height) =
                (div.offset_width() as f32, div.offset_height() as f32);

            let max_width =
                window_width.min(((current_width / current_height) as f64) * window_height);
            let expected_new_width: f64 = width.get_untracked();
            set_width.set(expected_new_width.min(max_width));
            let new_width = width.get_untracked();
            let new_height = ((current_height / current_width) as f64) * new_width;
            let max_left = window_width - new_width;
            let max_top = window_height - new_height;
            let (x, y): (f32, f32) = position.get_untracked();
            set_position.set((x.clamp(0.0, max_left as f32), y.clamp(0.0, max_top as f32)));
        }
    });

    view! {
        <Portal>
            <div ref=div_ref class="fixed flex flex-col rounded-md cursor-grab z-50 touch-none overflow-hidden"

                style=move||format!(
                    "left: {}px; top: {}px; width: {}px",
                    position.get().0,
                    position.get().1,
                    width.get()
                )

                on:pointerdown=pointer_down
                on:pointermove=pointer_move

                on:pointerup=pointer_up
                on:pointercancel=pointer_up
                on:pointerout=pointer_up
                on:pointerleave=pointer_up
            >
                <For
                    each=move||{
                        let users = video_users.get().keys().cloned().collect::<Vec<_>>();
                        users
                    }
                    key=|id|*id
                    let:user_id
                >
                    {
                        let user = create_memo(move |_| video_users.get_untracked().get(&user_id).cloned());
                        move ||{
                            if let Some(user) = user.get() {
                                let video_ref= user.video_ref;
                                let is_video_active = user.is_video_active;
                                view! {
                                    <video ref={video_ref}
                                        class="w-full"
                                        class=("hidden", move || !is_video_active.get())
                                    />
                                }.into_view()
                            }else{
                                view! {}.into_view()
                            }
                        }
                    }
                </For>
            </div>
        </Portal>
    }
}

#[component]
pub fn VideoChatManager(
    #[prop(into)] is_open: MaybeSignal<bool>,
    #[prop(into)] close: Callback<()>,
) -> impl IntoView {
    let rm = expect_context::<RoomManager>();

    #[derive(Clone, PartialEq, Eq)]
    struct VideoChatUser {
        pub meta: RwSignal<UserMeta>,
        pub is_self: bool,
    }

    let (video_users, set_video_users) = create_signal(HashMap::<Uuid, VideoChatUser>::new());

    let room_info = rm.get_room_info();

    let owner = Owner::current().expect("No owner");
    create_effect(move |_| {
        if let Some(room_info) = room_info.get() {
            let vu = video_users.get_untracked();
            let mut new_users = HashMap::new();
            for user in room_info.users {
                if let Some(user_v) = vu.get(&user.id) {
                    let user_id = user.id;
                    user_v.meta.set(user);
                    new_users.insert(user_id, user_v.clone());
                } else {
                    new_users.insert(
                        user.id,
                        VideoChatUser {
                            is_self: room_info.user_id == user.id,
                            meta: with_owner(owner, || create_rw_signal(user)),
                        },
                    );
                }
            }
            set_video_users.set(new_users);
        }
    });

    view! {
        <VideoChatConsent />

        <Show when=move||is_open.get()>
            <div class="fixed w-full top-0 left-0 h-full z-50 text-white bg-black/40 flex justify-center items-center">
                <div class="">
                    <Dialog
                        is_self_sized=true
                        is_open=true
                        on_close=move|_|{
                            close.call(());
                        }
                    >
                        <div class="text-center">
                            "Video/Audio Call"
                        </div>
                        <div class="h-4" />
                        <For
                            each=move||{
                                let ids = video_users.get().keys().cloned().collect::<Vec<_>>();
                                ids
                            }
                            key=|id|*id
                            let:user_id
                        >
                            {
                                let user = create_memo(move |_| video_users.get().get(&user_id).cloned());
                                if let Some(user) = user.get() {
                                    if user.is_self {
                                        view! {

                                        }.into_view()
                                    }else{
                                        view! {
                                            <div class="flex gap-4 items-center">
                                                <div class="text-lg"> { move || user.meta.get().name } </div>
                                                <div class="flex-grow" />
                                                <div class="flex gap-3">
                                                    <button class="w-8"
                                                        on:click=move|_|{
                                                            let rm = expect_context::<RoomManager>();
                                                            let toaster = expect_context::<Toaster>();
                                                            leptos::spawn_local(async move {
                                                                if let Err(err) =  rm.send_vc_request(user.meta.get_untracked().id, true, true).await{
                                                                    warn!("Failed to send vc request {err:?}");
                                                                    toaster.toast(Toast { message: "Failed to video call".into(), r#type: crate::components::toaster::ToastType::Failed });
                                                                }else{
                                                                    toaster.toast(Toast { message: "Sent video call request".into(), r#type: crate::components::toaster::ToastType::Success });
                                                                    close.call(());
                                                                }
                                                            });
                                                        }
                                                    >
                                                        <Icon icon=crate::components::icons::Icons::Video />
                                                    </button>
                                                    <button class="w-8"
                                                        on:click=move|_|{
                                                            let rm = expect_context::<RoomManager>();
                                                            let toaster = expect_context::<Toaster>();
                                                            leptos::spawn_local(async move {
                                                                if let Err(err) =  rm.send_vc_request(user.meta.get_untracked().id, false, true).await{
                                                                    warn!("Failed to send vc request {err:?}");
                                                                    toaster.toast(Toast { message: "Failed to audio call".into(), r#type: crate::components::toaster::ToastType::Failed });
                                                                }else{
                                                                    toaster.toast(Toast { message: "Sent auio call request".into(), r#type: crate::components::toaster::ToastType::Success });
                                                                    close.call(());
                                                                }
                                                            });
                                                        }
                                                    >
                                                        <Icon icon=crate::components::icons::Icons::Mic />
                                                    </button>
                                                </div>
                                            </div>
                                            <div class="h-4" />
                                        }.into_view()
                                    }
                                }else{
                                    view! {}.into_view()
                                }
                            }
                        </For>
                    </Dialog>
                </div>
            </div>
        </Show>
    }
}

#[component]
pub fn VideoChatConsent() -> impl IntoView {
    let rm = expect_context::<RoomManager>();
    let video_permission_req = rm.permission_request_signal;
    let (request, set_request) = create_signal(None);

    create_effect(move |_| {
        if let Some((user, video, audio)) = video_permission_req.get() {
            let user = rm
                .get_room_info()
                .with_untracked(|r| {
                    r.as_ref()
                        .map(|r| r.users.iter().find(|u| u.id == user).cloned())
                })
                .flatten();
            if let Some(user) = user {
                if video || audio {
                    set_request.set(Some((user, video, audio)));
                }
            }
        }
    });

    view! {
        {
            move || {
                if let Some(request) = request.get() {
                    view! {
                        <div class="fixed w-full top-0 left-0 h-full z-50 text-white bg-black/40 flex justify-center items-center">
                            <div class="">
                                <Dialog
                                    is_self_sized=true
                                    is_open=true
                                    on_close=move|_|{
                                        set_request.set(None);
                                    }
                                >
                                    <div class="text-center text-lg">
                                        {if request.1 { "Video"} else {"Audio"}} " Call Request"
                                    </div>
                                    <div class="h-4" />
                                    <div class="flex gap-2 text-center items-center justify-center">
                                        <span class="w-6">
                                            <Icon icon={if request.1 {crate::components::icons::Icons::Video}else {crate::components::icons::Icons::Mic}} />
                                        </span>
                                        <span>
                                            {request.0.name}
                                        </span>
                                    </div>
                                    <div class="h-6" />
                                    <div class="flex gap-4">
                                        <button
                                           class="text-sm hover:bg-white/20 self-center px-4 py-1"
                                            type="button"
                                            on:click=move|_|{
                                                let rm = expect_context::<RoomManager>();
                                                let toaster = expect_context::<Toaster>();
                                                leptos::spawn_local(async move {
                                                    let res = rm.connect_audio_chat(request.0.id, request.1, request.2).await;
                                                    if let Err(err) = res {
                                                        toaster.toast(Toast{
                                                            message: format!("{err:?}").into(),
                                                            r#type:crate::components::toaster::ToastType::Failed
                                                        });
                                                    }
                                                });
                                            }
                                        >
                                            "[ Accept ]"
                                        </button>

                                        <button
                                           class="text-sm hover:bg-white/20 self-center px-4 py-1"
                                            type="button"
                                            on:click=move|_|{
                                                set_request.set(None);
                                            }
                                        >
                                            "[ Reject ]"
                                        </button>
                                    </div>

                                </Dialog>
                            </div>
                        </div>
                    }.into_view()
                }else{
                    view! {}.into_view()
                }
            }
        }
    }
}
