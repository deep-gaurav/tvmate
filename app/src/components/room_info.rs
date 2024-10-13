use leptos::*;
use tracing::warn;
use web_sys::ShareData;

use crate::components::icons::Icon;
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
                                    view! {
                                        <div
                                            class="text-left w-full mt-2 break-words"
                                        >
                                            "> "
                                            {user.name}
                                            {match user.state {
                                                common::UserState::VideoNotSelected => "⌛",
                                                common::UserState::VideoSelected(_) => "✔️",
                                            }}
                                        </div>
                                    }
                                })
                                .collect::<Vec<_>>()
                        }}
                        <div class="h-3" />
                        <hr class="border-white border-t w-full" />
                        <div class="h-3" />
                        {
                            move || {
                                let room_id = room_info
                                .with(|r| r.as_ref().map(|r| r.id.to_lowercase()));
                                if room_id.is_some() {
                                    view! {
                                        <button class="flex gap-2 items-center text-sm"
                                            on:click=move|_|{
                                                let url = window().location().href();
                                                if let Ok(url) = url {
                                                    let navigator = window().navigator();
                                                    let share =  navigator.share_with_data(&{
                                                        let share_data = ShareData::new();
                                                        share_data.set_url(&url);
                                                        share_data.set_title("Let's have a watch party together, join me on TVMate with following link.");
                                                        share_data
                                                    });
                                                    let wasm_fut = wasm_bindgen_futures::JsFuture::from(share);
                                                    leptos::spawn_local(async move {
                                                        if let Err(err) = wasm_fut.await {
                                                            warn!("Cannot share link {err:?}");
                                                        }
                                                    });
                                                }else{
                                                    warn!("Cant get url")
                                                }
                                            }
                                        >
                                            <Icon class="w-6" icon=crate::components::icons::Icons::Share />
                                            <span>
                                                "Invite"
                                            </span>
                                        </button>
                                    }.into_view()
                                }else{
                                    view! {}.into_view()
                                }
                            }
                        }
                    </Portal>
                }
                    .into_view()
            } else {
                view! {}.into_view()
            }
        }}
    }
}
