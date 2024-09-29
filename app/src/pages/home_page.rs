use ev::Event;
use leptos::component;
use leptos::*;
use leptos_use::use_event_listener;
use logging::warn;
use tracing::info;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::js_sys;

use crate::components::dialog::Dialog;
use crate::networking::room_manager::RoomManager;

/// Renders the home page of your application.
#[component]
pub fn HomePage() -> impl IntoView {
    let (host_open, set_host_open) = create_signal(false);
    let (join_open, set_join_open) = create_signal(false);

    let (install_prompt, set_install_prompt) = create_signal(None);
    create_effect(move |_| {
        let _ = use_event_listener(
            window(),
            ev::Custom::new("beforeinstallprompt"),
            move |ev: Event| {
                ev.prevent_default();
                info!("Before install prompt fired");
                set_install_prompt.set(Some(ev));
            },
        );
    });

    view! {
        <Dialog
            is_open=host_open
            on_close=move |_| {
                set_host_open.set(false);
            }
        >
            {{
                let (name, set_name) = create_signal(String::new());
                view! {
                    <h3 class="font-bold2  text-xl text-center w-full">"Host"</h3>

                    <div class="h-4" />

                    <div class="flex items-center">
                        <label class=" font-thin8 text-sm" for="name">
                            "Name: "
                        </label>
                        <input
                            class="bg-white/10 focus:outline-white/50  text-md font-thin8 p-2"
                            name="name"
                            type="text"
                            placeholder="Enter your name"
                            on:input=move |ev| {
                                set_name.set(event_target_value(&ev));
                            }
                        />
                    </div>

                    <div class="h-4" />

                    <button
                        class="text-sm hover:bg-white/20 self-center px-4 py-1"
                        type="button"
                        on:click=move |_| {
                            if name.get_untracked().is_empty() {
                                warn!("Name cant be empty");
                            } else {
                                let room_manager = expect_context::<RoomManager>();
                                if let Err(err) = room_manager.host_join(name.get_untracked(), None)
                                {
                                    warn!("Cannot join {err:#?}");
                                }
                            }
                        }
                    >
                        "[ Create Room ]"
                    </button>
                }
            }}
        </Dialog>

        <Dialog
            is_open=join_open
            on_close=move |_| {
                set_join_open.set(false);
            }
        >
            {
                let (name, set_name) = create_signal(String::new());
                let (room_code, set_room_code) = create_signal(String::new());
                view! {
                    <h3 class="font-bold2  text-xl text-center w-full">"Join"</h3>

                    <div class="h-4" />

                    <div class="flex items-center">
                        <label class=" font-thin8 text-sm" for="name">
                            "Name: "
                        </label>
                        <input
                            class="bg-white/10 focus:outline-white/50  text-md font-thin8 p-2"
                            name="name"
                            type="text"
                            placeholder="Enter your name"
                            on:input=move |ev| {
                                set_name.set(event_target_value(&ev));
                            }
                        />
                    </div>

                    <div class="flex items-center">
                        <label class=" font-thin8 text-sm" for="roomid">
                            "Room Id: "
                        </label>
                        <input
                            class="bg-white/10 focus:outline-white/50  text-md font-thin8 p-2"
                            name="roomid"
                            type="text"
                            placeholder="Room Id"
                            on:input=move |ev| {
                                set_room_code.set(event_target_value(&ev));
                            }
                        />
                    </div>

                    <div class="h-4" />

                    <button
                        class="text-sm hover:bg-white/20 self-center px-4 py-1"
                        type="button"
                        on:click=move |_| {
                            if name.get_untracked().is_empty()
                                || room_code.get_untracked().is_empty()
                            {
                                warn!("Name cant be empty");
                            } else {
                                let room_manager = expect_context::<RoomManager>();
                                if let Err(err) = room_manager
                                    .host_join(
                                        name.get_untracked(),
                                        Some(room_code.get_untracked()),
                                    )
                                {
                                    warn!("Cannot join {err:#?}");
                                }
                            }
                        }
                    >
                        "[ Join Room ]"
                    </button>
                }
            }
        </Dialog>
        <div class="h-full w-full flex flex-col items-center justify-center ">

            <div class="flex-grow" />
            <h1 class="font-bold2 text-xl">"Welcome to TVMate"</h1>
            <div class="h-4" />
            <div class="flex gap-4">
                <button
                    class="font-bold1 text-lg"
                    on:click=move |_| {
                        set_host_open.set(true);
                    }
                >
                    "[ Host ]"
                </button>
                <button class="font-bold1 text-lg" on:click=move |_| set_join_open.set(true)>
                    "[ Join ]"
                </button>
            </div>
            <div class="flex-grow" />

            {move || {
                if let Some(prompt_event) = install_prompt.get() {
                    view! {
                        <button
                            class="font-bold1 text-sm"
                            on:click=move |_| {
                                let _ = js_sys::Reflect::get(
                                        &prompt_event,
                                        &JsValue::from_str("prompt"),
                                    )
                                    .expect("Failed to get 'prompt' property")
                                    .dyn_ref::<js_sys::Function>()
                                    .expect("'prompt' is not a function")
                                    .call0(&prompt_event)
                                    .expect("Failed to call 'prompt' function");
                            }
                        >
                            "[ Install Web App ]"
                        </button>
                        <div class="h-4" />
                    }
                        .into_view()
                } else {
                    view! {}.into_view()
                }
            }}
        </div>
    }
}
