use leptos::*;
use tracing::info;

use crate::{components::portal::Portal, networking::room_manager::RoomManager, MountPoints};

#[component]
pub fn ChatBox() -> impl IntoView {
    let room_manager = expect_context::<RoomManager>();

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
                            let el = mount_points.side_point.get();
                            if let Some(el) = el {
                                let element: &web_sys::Element = el.as_ref();
                                let element = element.clone();
                                info!("Mounting to portal");
                                view! {
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
