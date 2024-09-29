use html::Audio;
use leptos::*;
use logging::warn;
use tracing::info;

use crate::components::portal::Portal;
use crate::networking::room_manager::RoomManager;
use crate::MountPoints;

#[component]
pub fn RoomInfo() -> impl IntoView {
    let room_manager = expect_context::<RoomManager>();
    let room_info = room_manager.get_room_info();
    view! {
        {move || {
            let mount_points = expect_context::<MountPoints>();
            let el = mount_points.handle_point.get();
            if let Some(el) = el {
                let element: &web_sys::Element = el.as_ref();
                let element = element.clone();
                view! {
                    <Portal
                        mount=element
                        class="h-full w-full bg-black text-white flex flex-col justify-stretch items-center overflow-auto"
                    >
                        <div class="text-xs font-thin8 text-center">"Room"</div>
                        <div class="break-words">
                            {move || match room_info
                                .with(|r| r.as_ref().map(|r| r.id.to_uppercase()))
                            {
                                Some(id) => id,
                                None => "Disconnected".to_string(),
                            }}
                        </div>
                        <hr class="border-white border-t w-full" />

                        {move || {
                            room_info
                                .with(|r| r.as_ref().map(|r| r.users.clone()))
                                .unwrap_or_default()
                                .into_iter()
                                .map(|user| {
                                    let audio_ref = create_node_ref::<Audio>();
                                    let audio_receiver = expect_context::<RoomManager>()
                                        .audio_chat_stream_signal
                                        .0;
                                    create_effect(move |_| {
                                        if let Some((user_id, stream)) = audio_receiver.get() {
                                            if user.id == user_id {
                                                if let Some(audio) = audio_ref.get_untracked() {
                                                    info!("Set audio source");
                                                    audio.set_src_object(Some(&stream));
                                                    if let Err(err) =  audio.play() {
                                                        warn!("Cannot play audio")
                                                    }
                                                }
                                            }
                                        }
                                    });
                                    view! {
                                        <button
                                            on:click=move |_| {
                                                let rm = expect_context::<RoomManager>();
                                                leptos::spawn_local(async move {
                                                    rm.connect_audio_chat(user.id).await;
                                                });
                                            }
                                            class="text-left w-full mt-2 break-words"
                                        >
                                            "> "
                                            {user.name}
                                            {match user.state {
                                                common::UserState::VideoNotSelected => "⌛",
                                                common::UserState::VideoSelected(_) => "✔️",
                                            }}
                                            <audio ref=audio_ref class="hidden" />
                                        </button>
                                    }
                                })
                                .collect::<Vec<_>>()
                        }}
                    </Portal>
                }
                    .into_view()
            } else {
                view! {}.into_view()
            }
        }}
    }
}
