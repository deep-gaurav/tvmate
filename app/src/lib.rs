use std::borrow::Cow;

use crate::error_template::{AppError, ErrorTemplate};

use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use leptos_use::{use_window_size, UseWindowSizeReturn};
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
    pub side_point: NodeRef<leptos::html::Div>,
    pub speaker_point: NodeRef<leptos::html::Div>,
}

#[derive(Clone)]
pub struct Endpoint {
    pub main_endpoint: Cow<'static, str>,
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let room_manager = RoomManager::new(Owner::current().unwrap());

    provide_context(room_manager);

    let handle_point = create_node_ref();
    let side_point = create_node_ref();
    let speaker_point = create_node_ref();

    let mount_points = MountPoints {
        handle_point,
        side_point,
        speaker_point,
    };

    provide_context(mount_points);

    let UseWindowSizeReturn { width, height } = use_window_size();

    let is_landscape = create_memo(move |_| width.get() / height.get() > 1042.0 / 751.0);

    view! {
        {
            cfg_if! {
                if #[cfg(not(feature="csr"))] {
                view!{
                    <Stylesheet id="leptos" href="/pkg/tvmate.css" />
                }
            }
            }
        }

        // sets the document title
        <Title text="Welcome to TVMate" />

        <Meta
            name="viewport"
            content="width=device-width, initial-scale=1, interactive-widget=resizes-content"
        />

        <Link rel="manifest" href="manifest.json" />

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors /> }.into_view()
        }>
            <main
                class="bg-black h-full w-full flex justify-center items-center text-white font-thin8"
                style=move || {
                    if is_landscape.get() {
                        "flex-direction:row;"
                    } else {
                        "flex-direction:column;"
                    }
                }
            >
                <div
                    class="relative aspect-[1042/751] flex-shrink-0"
                    style=move || { if is_landscape.get() { "height:100%" } else { "width:100%" } }
                >
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

                    <div
                        class="absolute left-[81.5%] w-[16%] top-[48%] h-[48%] z-30"
                        ref=speaker_point
                    ></div>
                </div>

                <div ref=side_point></div>
            </main>
        </Router>
    }
}
