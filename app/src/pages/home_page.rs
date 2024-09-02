use leptos::component;
use leptos::*;
use logging::warn;

use crate::components::dialog::Dialog;
use crate::networking::room_manager::RoomManager;

/// Renders the home page of your application.
#[component]
pub fn HomePage() -> impl IntoView {
    let (host_open, set_host_open) = create_signal(false);
    let (join_open, set_join_open) = create_signal(false);

    view! {
        <Dialog is_open=host_open on_close=move|_|{
            set_host_open.set(false);
        }>
            {
                move || {
                    let (name, set_name) = create_signal(String::new());
                    view! {
                        <h3 class="font-bold2  text-xl text-center w-full"> "Host" </h3>

                        <div class="h-4" />

                        <div class="flex items-center">
                            <label class=" font-thin8 text-sm" for="name"> "Name: " </label>
                            <input class="bg-white/10 focus:outline-white/50  text-md font-thin8 p-2" name="name" type="text" placeholder="Enter your name"
                                on:input=move|ev| {
                                    set_name.set(event_target_value(&ev));
                                }
                            />
                        </div>

                        <div class="h-4" />

                        <button class="text-sm hover:bg-white/20 self-center px-4 py-1"
                            type="button"
                            on:click=move|_|{
                                if name.get_untracked().is_empty() {
                                    // TODO: add toast
                                    warn!("Name cant be empty");
                                }else{
                                    let room_manager = expect_context::<RoomManager>();
                                    if let Err(err) =  room_manager.host(name.get_untracked()) {
                                        warn!("Cannot join {err:#?}");
                                        //TODO: add toast
                                    }
                                }
                            }
                        > "[ Create Room ]" </button>
                    }
                }
            }
        </Dialog>

        <Dialog is_open=join_open on_close=move|_|{
            set_join_open.set(false);
        }>
            <h3 class="font-bold2  text-xl text-center w-full"> "Join" </h3>

            <div class="h-4" />

            <div class="flex items-center">
                <label class=" font-thin8 text-sm" for="name"> "Name: " </label>
                <input class="bg-white/10 focus:outline-white/50  text-md font-thin8 p-2" name="name" type="text" placeholder="Enter your name"/>
            </div>

            <div class="flex items-center">
                <label class=" font-thin8 text-sm" for="roomid"> "Room Id: " </label>
                <input class="bg-white/10 focus:outline-white/50  text-md font-thin8 p-2" name="roomid" type="text" placeholder="Room Id"/>
            </div>

            <div class="h-4" />

            <button class="text-sm hover:bg-white/20 self-center px-4 py-1"

            > "[ Create Room ]" </button>
        </Dialog>
        <div class="h-full w-full flex flex-col items-center justify-center ">
            <h1 class="font-bold2 text-xl"> "Welcome to SyncedCRT" </h1>
            <div class="h-4" />
            <div class="flex gap-4">
                <button class="font-bold1 text-lg"
                    on:click=move|_|{
                        set_host_open.set(true);
                    }
                > "[ Host ]" </button>
                <button class="font-bold1 text-lg"
                    on:click=move|_|set_join_open.set(true)
                > "[ Join ]" </button>
            </div>

        </div>
    }
}
