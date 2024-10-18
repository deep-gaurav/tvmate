use codee::string::FromToStringCodec;
use leptos::*;
use leptos_use::storage::use_local_storage;

use crate::components::{dialog::Dialog, icons::Icon};

#[component]
pub fn IntroHelpDialog() -> impl IntoView {
    let (intro_help_dismissed, set_intro_help_dismissed, _delete_storage) =
        use_local_storage::<bool, FromToStringCodec>("intro_help_dismissed");

    let (is_open, set_is_open) = create_signal(!intro_help_dismissed.get_untracked());

    let (is_help_overlay_open, set_is_help_overlay_open) = create_signal(false);

    view! {
        <div class="fixed top-4 right-4 z-[60]">
            <button type="button"
                class="w-10 md:w-6"
                class=("hidden", is_open)

                on:click=move|_|{
                    set_is_open.set(true);
                }
            >
                <Icon icon=crate::components::icons::Icons::Help />
            </button>
        </div>
        <div class="fixed top-0 left-0 bg-black/90 z-50 h-full w-full"
            class=("hidden", move || !is_help_overlay_open.get())
        >
            <div class="absolute right-12 top-12 w-20">
                <Icon icon=crate::components::icons::Icons::ArrowUpRight />
            </div>
            <div class="absolute w-full px-32 top-32 text-right flex flex-col gap-4">
                <div class="text-sm"> "You can always access help on corner" </div>
                <button
                    class=" hover:bg-white/20 self-end px-4 py-1"
                    type="button"
                    on:click=move|_|{
                        set_is_help_overlay_open.set(false);
                    }
                >
                    "[ Got it ]"
                </button>
            </div>
        </div>
        <Show when=move||is_open.get()>
            <div class="fixed bg-black/50 top-0 left-0 h-full w-full flex items-center justify-center p-6 z-50">
                <div class="lg:max-w-[60%] h-fit max-h-full overflow-auto text-white font-bold1">
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
                                class=("hidden", move || !intro_help_dismissed.get())
                                type="button"
                                on:click=move|_|{
                                    set_is_open.set(false);
                                }
                            >
                                "[ Close ]"
                            </button>

                            <button
                                class="text-xl hover:bg-white/20 self-center px-4 py-1"
                                class=("hidden", move || intro_help_dismissed.get())
                                type="button"
                                on:click=move|_|{
                                    set_is_open.set(false);
                                    set_intro_help_dismissed.set(true);
                                    set_is_help_overlay_open.set(true);
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

#[component]
pub fn RoomHelpDialog() -> impl IntoView {
    let (intro_help_dismissed, set_intro_help_dismissed, _delete_storage) =
        use_local_storage::<bool, FromToStringCodec>("room_help_dismissed");

    let (is_open, set_is_open) = create_signal(!intro_help_dismissed.get_untracked());

    view! {
        <div class="fixed top-4 right-4 z-[60]">
            <button type="button"
                class="w-10 md:w-6"
                class=("hidden", is_open)
                on:click=move|_|{
                    set_is_open.set(true);
                }
            >
                <Icon icon=crate::components::icons::Icons::Help />
            </button>
        </div>
        <Show when=move||is_open.get()>
            <div class="fixed bg-black/50 top-0 left-0 h-full w-full flex items-center justify-center p-6 z-50">
                <div class="lg:max-w-[60%] h-fit max-h-full overflow-auto text-white font-bold1">
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
                                "You're now in a TVMate room! Here's what you can do:"
                            </div>
                            <div class="h-4" />
                            <ul class="text-xl list-disc flex flex-col gap-2 list-outside pl-2">
                                <li> <span class="text-2xl"> "Select a Video: " </span> "Either drag and drop a video file into the TV screen or click 'Select Video' to pick one from your device. Your video will sync with others in the room." </li>
                                <li> <span class="text-2xl"> "Invite Friends: " </span> "Copy and share the room link using the Invite button on the panel." </li>
                                <li> <span class="text-2xl"> "Video/Audio Call: " </span> "Use the Video Call button to start a call with others in the room." </li>
                                <li> <span class="text-2xl"> "Sync Options: " </span> "If someone else has already selected a video, you can choose to stream their video instead of selecting your own." </li>
                            </ul>
                            <div class="h-4" />
                            <div>
                                "Enjoy watching together!"
                            </div>

                            <div class="h-8" />

                            <button
                                class="text-xl hover:bg-white/20 self-center px-4 py-1"
                                class=("hidden", move || !intro_help_dismissed.get())
                                type="button"
                                on:click=move|_|{
                                    set_is_open.set(false);
                                }
                            >
                                "[ Close ]"
                            </button>

                            <button
                                class="text-xl hover:bg-white/20 self-center px-4 py-1"
                                class=("hidden", move || intro_help_dismissed.get())
                                type="button"
                                on:click=move|_|{
                                    set_is_open.set(false);
                                    set_intro_help_dismissed.set(true);
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
