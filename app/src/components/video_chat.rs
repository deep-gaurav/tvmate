use std::{collections::HashMap, hash::Hash};

use common::UserMeta;
use ev::PointerEvent;
use leptos::*;
use tracing::{info, warn};
use uuid::Uuid;

use crate::networking::room_manager::RoomManager;

#[derive(Clone)]
struct VideoUser {
    user_meta: RwSignal<UserMeta>,
    video_ref: NodeRef<leptos::html::Video>,
    is_video_active: RwSignal<bool>,
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
                user_meta,
                video_ref,
                is_video_active,
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

    let (is_mouse_down, set_is_mouse_down) = create_signal(false);
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

    let pointer_move = move |ev: PointerEvent| {
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
                set_width.set(width.get_untracked() + (current_diff - diff));
            }
            diff.update_value(|d| *d = Some(current_diff));
        } else if let Some(ev) = pointer_events.with_value(|p| {
            let mut it = p.values();
            it.next().cloned()
        }) {
            let (x, y) = position.get_untracked();
            set_position.set((x - ev.movement_x() as f32, y - ev.movement_y() as f32));
        }
    };

    view! {
        <div class="fixed flex flex-col rounded-md cursor-grab z-50"

            style=move||format!(
                "right: {}px; bottom: {}px; width: {}px",
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
                    move ||{
                        if let Some(user) = video_users.get_untracked().get(&user_id){
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
    }
}
