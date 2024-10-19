use leptos::*;
use logging::warn;
use tracing::info;
use web_sys::ShareData;

use crate::{
    components::{icons::Icon, portal::Portal},
    networking::room_manager::RoomManager,
    MountPoints,
};

#[component]
pub fn ChatBox() -> impl IntoView {
    let room_manager = expect_context::<RoomManager>();

    let room_id = create_memo({
        let rm = room_manager.get_room_info();
        move |_| {
            if let Some(info) = rm.with(|r| r.as_ref().map(|r| r.id.to_uppercase())) {
                info
            } else {
                "".to_string()
            }
        }
    });
    let is_connected = create_memo({
        let info = room_manager.get_room_info();
        move |_| info.get().is_some()
    });
    view! {
        {
            move || {
                let _ = is_connected.get();
                if let Some((message_signal, message_history)) = room_manager.get_chat_signal() {
                    let (msg_len, set_msg_len) = create_signal(message_history.with_value(|v| v.len()));

                    create_effect(move |_| {
                        message_signal.with(|_| ());
                        set_msg_len.set(message_history.with_value(|v| v.len()));
                    });
                    let (chat_msg, set_chat_msg) = create_signal(String::new());

                    view! {
                        {move || {
                            let mount_points = expect_context::<MountPoints>();
                            let (el, el2) = (mount_points.side_point.get(), mount_points.side_point_2.get());
                            if let (Some(el), Some(el2)) = (el, el2) {
                                let element: &web_sys::Element = el.as_ref();
                                let element2: &web_sys::Element = el2.as_ref();
                                let element = element.clone();
                                let element2 = element2.clone();
                                info!("Mounting to portal");
                                view! {

                                    <Portal
                                        mount=element2
                                        mount_class="md:hidden w-full"
                                        class="h-16 w-full flex flex-col justify-stretch"
                                    >
                                        <div class="flex gap-4 items-center justify-center h-full">
                                            <div> "Room " {move || room_id.get()} </div>

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
                                            </button>
                                        </div>
                                    </Portal>
                                    <Portal
                                        mount=element
                                        mount_class="h-full w-full"
                                        class="h-full w-full flex flex-col justify-stretch"
                                    >
                                        <div class="text-center w-full">"Chat"</div>
                                        // <hr class="border-white border-t w-full" />

                                        <div class="flex-grow h-0 overflow-auto w-full flex flex-col-reverse">
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
                                                            <div class="w-full text-md font-thin14">
                                                                <span class="font-thin8 text-md">{user.name} ": "</span>
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
                                        <form
                                            class="w-full flex"
                                            on:submit=move |ev| {
                                                ev.prevent_default();
                                                let rm = expect_context::<RoomManager>();
                                                rm.send_chat(chat_msg.get_untracked());
                                                set_chat_msg.set(String::new());
                                            }
                                        >
                                            <input
                                                class="w-full text-kg font-thin16 p-2 bg-transparent text-white"
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
                                            <button class="p-3 border text-kg font-thin14 ">"Submit"</button>
                                        </form>
                                    </Portal>
                                }
                                    .into_view()
                            } else {
                                info!("Side point not mounted");
                                view! {}.into_view()
                            }
                        }}
                    }.into_view()
                } else {
                    view! {}.into_view()
                }
            }
        }
    }
}
