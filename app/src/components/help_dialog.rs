use leptos::*;

use crate::components::dialog::Dialog;

#[component]
pub fn HelpDialog() -> impl IntoView {
    let (is_open, set_is_open) = create_signal(true);
    view! {
        <Show when=move||is_open.get()>
            <div class="fixed bg-black/50 top-0 left-0 h-full w-full flex items-center justify-center p-6 z-50">
                <div class="lg:max-w-[60%] h-fit text-white font-bold1">
                    <Dialog
                        is_self_sized=true
                        is_open=true
                        on_close=move|_|{
                            set_is_open.set(false);
                        }
                    >
                        <div class="flex flex-col text-xl">
                            <div class="font-thin8 text-3xl
                                flex flex-row gap-2 items-center justify-center
                            "
                            >
                                "Instructions"
                            </div>
                            <div class="h-8" />
                            <div class="">
                                "Welcome to TVMate! You can either Host or Join a room to start watching videos together."
                            </div>
                            <div class="h-4" />
                            <ul class="text-xl list-disc flex flex-col gap-2 list-outside pl-2">
                                <li> <span class="text-2xl"> "Host: " </span> "Click to create a new room. You’ll need to enter your name, and a room will be created for you." </li>
                                <li> <span class="text-2xl"> "Join: " </span> "Enter a room code and your name to join an existing room." </li>
                            </ul>
                            <div class="h-4" />
                            <div>
                                "Once you're in a room, you can select a video, sync playback, chat, and make video or audio calls"
                            </div>

                            <div class="h-8" />

                            <button
                                class="text-xl hover:bg-white/20 self-center px-4 py-1"
                                type="button"
                                on:click=move|_|{
                                    set_is_open.set(false);
                                }
                            >
                                "[ Got it, Don't show again ]"
                            </button>

                            <div class="h-4" />
                        </div>

                    </Dialog>
                </div>
            </div>
        </Show>
    }
}
