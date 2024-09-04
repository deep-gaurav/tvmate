use crate::error_template::{AppError, ErrorTemplate};

use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use networking::room_manager::RoomManager;
use pages::room::RoomPage;

use crate::pages::home_page::HomePage;

pub mod components;
pub mod error_template;
pub mod networking;
pub mod pages;

#[derive(Clone)]
pub struct MountPoints {
    pub handle_point: NodeRef<leptos::html::Div>,
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let room_manager = RoomManager::new(Owner::current().unwrap());

    provide_context(room_manager);

    let handle_point = create_node_ref();

    let mount_points = MountPoints { handle_point };

    provide_context(mount_points);

    view! {
        <Stylesheet id="leptos" href="/pkg/syncedcrt.css" />

        // sets the document title
        <Title text="Welcome to SyncedCRT" />

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors /> }.into_view()
        }>
            <main class="bg-black h-full w-full flex justify-center items-center main-cont text-white font-thin8">
                <div class="relative tv-cont aspect-[1042/751] ">
                    <div class="h-full w-full absolute bg-cover bg-center bg-no-repeat bg-[url('/assets/images/synced_crt.png')] z-10 pointer-events-none" />
                    <div class="absolute left-[7%] w-[68%] top-[11%] h-[79%] bg-slate-800">
                        <Routes>
                            <Route path="" view=HomePage />
                            <Route path="room/:id" view=RoomPage />
                        </Routes>
                    </div>
                    <div
                        class="absolute left-[81.5%] w-[16%] top-[6%] h-[30%] z-20"
                        ref=handle_point
                    ></div>
                </div>
            </main>
        </Router>
    }
}
