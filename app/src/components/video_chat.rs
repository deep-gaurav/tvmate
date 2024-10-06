use std::{collections::HashMap, hash::Hash};

use common::UserMeta;
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
    let touch_events = store_value(HashMap::new());

    view! {
        <div class="fixed flex flex-col rounded-md cursor-grab z-50"

            style=move||format!(
                "right: {}px; bottom: {}px; width: {}px",
                position.get().0,
                position.get().1,
                width.get()
            )

            on:mousedown=move|_|{
                set_is_mouse_down.set(true);
            }
            on:mouseup=move|_|{
                set_is_mouse_down.set(false);
            }
            on:mouseleave=move|_|{
                set_is_mouse_down.set(false);
            }
            on:touchstart=move|ev|{
                let changed_touches = ev.changed_touches();
                for i in 0..changed_touches.length() {
                    if let Some(touch) = changed_touches.get(i) {
                        touch_events.update_value(|touches|{
                            touches.insert(touch.identifier(), (touch.page_x(), touch.page_y()));
                        });
                    }
                }
            }
            on:touchend=move|ev|{
                let changed_touches = ev.changed_touches();
                for i in 0..changed_touches.length() {
                    if let Some(touch) = changed_touches.get(i) {
                        touch_events.update_value(|touches|{
                            touches.remove(&touch.identifier());
                        });
                    }
                }
            }
            on:touchcancel=move|ev|{
                let changed_touches = ev.changed_touches();
                for i in 0..changed_touches.length() {
                    if let Some(touch) = changed_touches.get(i) {
                        touch_events.update_value(|touches|{
                            touches.remove(&touch.identifier());
                        });
                    }
                }
            }
            on:touchmove=move|ev|{
                let changed_touches = ev.changed_touches();
                if let Some(previous_scale) = touch_events.with_value(|touchs|{
                    if touchs.len() != 2 {
                        None
                    }else{
                        let mut tc = touchs.iter();
                        let (_,p1) = tc.next().expect("First touch not present");
                        let (_,p2) = tc.next().expect("Second touch not present");
                        let dist = (((p1.0 as f64).powi(2) - (p2.0 as f64).powi(2))/((p1.1 as f64).powi(2) - (p2.1 as f64).powi(2))).sqrt();

                        Some(dist)
                    }
                }){

                    for i in 0..changed_touches.length() {
                        if let Some(touch) = changed_touches.get(i) {
                            touch_events.update_value(|touches|{
                                touches.insert(touch.identifier(), (touch.page_x(), touch.page_y()));
                            });
                        }
                    }

                    if let Some(new_scale) = touch_events.with_value(|touchs|{
                        if touchs.len() != 2 {
                            None
                        }else{
                            let mut tc = touchs.iter();
                            let (_,p1) = tc.next().expect("First touch not present");
                            let (_,p2) = tc.next().expect("Second touch not present");
                            let dist = (((p1.0 as f64).powi(2) - (p2.0 as f64).powi(2))/((p1.1 as f64).powi(2) - (p2.1 as f64).powi(2))).sqrt();

                            Some(dist)
                        }
                    }){
                        set_width.set(width.get_untracked()+new_scale-previous_scale);
                    }


                }
                for i in 0..changed_touches.length() {
                    if let Some(touch) = changed_touches.get(i) {
                        if let Some(previous_touch) = touch_events.with_value(|touches|touches.get(&touch.identifier()).cloned()){
                            let (x,y) = position.get_untracked();
                            set_position.set(
                                (x-(touch.page_x() as f32 - previous_touch.0 as f32), y-(touch.page_y() as f32 - previous_touch.1 as f32))
                            );
                            touch_events.update_value(|touches|{
                                touches.insert(touch.identifier(), (touch.page_x(),touch.page_y()));
                            })
                        }
                    }
                }
            }
            on:mousemove=move|ev|{
                if is_mouse_down.get_untracked() {
                    let (x,y) = position.get_untracked();
                    set_position.set(
                        (x-ev.movement_x() as f32, y-ev.movement_y() as f32)
                    );
                }
            }
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
