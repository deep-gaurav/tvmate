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
        <Stylesheet id="leptos" href="/pkg/syncedcrt.css"/>

        // sets the document title
        <Title text="Welcome to SyncedCRT"/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <main class="bg-black h-full w-full flex justify-center items-center main-cont text-white font-thin8">
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
    let (host_open, set_host_open) = create_signal(false);
    view! {
        <div class="absolute h-full w-full bg-black/30 flex items-center justify-center p-10"
            class=("hidden", move || !host_open.get())
        >
            <div class="p-1 bg-black relative shadow-box shadow-white/45 ">
                <button class=" absolute right-6 -top-1 bg-black text-sm font-bold1"
                    on:click=move|_|{
                        set_host_open.set(false);
                    }
                > " [ x ] " </button>
                <div class="border border-white p-0.5">
                <div class="border border-white p-4 flex flex-col">
                    <h3 class="font-bold2  text-xl text-center w-full"> "Host" </h3>

                    <div class="h-4" />

                    <div class="flex items-center">
                        <label class=" font-thin8 text-sm" for="name"> "Name: " </label>
                        <input class="bg-white/10 focus:outline-white/50  text-md font-thin8 p-2" name="name" type="text" placeholder="Enter your name"/>
                    </div>

                    <div class="h-4" />

                    <button class="text-sm hover:bg-white/20 self-center px-4 py-1"> "[ Create Room ]" </button>
                </div>
                </div>
            </div>
        </div>
        <div class="h-full w-full flex flex-col items-center justify-center ">
            <h1 class="font-bold2 text-xl"> "Welcome to SyncedCRT" </h1>
            <div class="h-4" />
            <div class="flex gap-4">
                <button class="font-bold1 text-lg"
                    on:click=move|_|{
                        set_host_open.set(true);
                    }
                > "[ Host ]" </button>
                <button class="font-bold1 text-lg"> "[ Join ]" </button>
            </div>

        </div>
    }
}
