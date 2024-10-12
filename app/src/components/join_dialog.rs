use leptos::*;
use tracing::{info, warn};

use crate::{components::dialog::Dialog, networking::room_manager::RoomManager};

#[component]
pub fn JoinDialog(
    #[prop(into)] is_open: MaybeSignal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] init_room_code: MaybeSignal<String>,
) -> impl IntoView {
    view! {
        <Dialog
            is_self_sized=false
            is_open=is_open
            on_close=move |_| {
                on_close.call(());
            }
        >
            {
                let (name, set_name) = create_signal(String::new());
                let (room_code, set_room_code) = create_signal(init_room_code.get_untracked());
                create_effect(move|_|{
                    set_room_code.set(init_room_code.get());
                });
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
                            prop:value=room_code
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
    }
}
