use leptos::*;

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
                    </Portal>
                }
                    .into_view()
            } else {
                view! {}.into_view()
            }
        }}
    }
}
