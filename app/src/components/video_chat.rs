use std::collections::HashMap;

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

    view! {
        <div class="fixed right-4 bottom-4 w-40 select-none pointer-events-none flex flex-col">
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
                                    class="w-full h-40"
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
