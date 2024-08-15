use crate::error_template::{AppError, ErrorTemplate};

use leptos::*;
use leptos_meta::*;
use leptos_router::*;

pub mod error_template;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/start-axum-workspace.css"/>

        // sets the document title
        <Title text="Welcome to SyncedCRT"/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <main class="bg-black h-full w-full flex justify-center items-center main-cont">
                <div class="relative tv-cont aspect-[1042/751] ">
                    <div class="h-full w-full absolute bg-cover bg-center bg-no-repeat bg-[url('/assets/images/synced_crt.png')] z-10 pointer-events-none" />
                    <div class="absolute left-[7%] w-[68%] top-[11%] h-[79%] bg-slate-800" >
                        <Routes>
                            <Route path="" view=HomePage/>
                        </Routes>
                    </div>
                </div>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div class="h-full w-full flex items-center justify-center text-white">
            <h1 class="font-bold2 text-xl"> "Welcome to SyncedCRT" </h1>
        </div>
    }
}
