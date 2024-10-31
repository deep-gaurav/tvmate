use app::{
    tauri_provider::{FullScreenProvider, ShareRequest},
    utils::StringWriter,
    App, Endpoint, LogProvider,
};
use futures::FutureExt;
use leptos::*;
use serde::{Deserialize, Serialize};
use tracing::{info, level_filters::LevelFilter, subscriber::set_global_default};
use tracing_subscriber::{layer::SubscriberExt, Layer};
use wasm_bindgen::JsValue;

fn main() {
    console_error_panic_hook::set_once();
    // initializes logging using the `log` crate
    console_error_panic_hook::set_once();

    use tracing_subscriber::fmt;
    use tracing_subscriber_wasm::MakeConsoleWriter;

    let logs = StoredValue::new(String::new());

    let string_writer = StringWriter { log_buffer: logs };

    let console_layer = fmt::layer()
        .with_writer(
            // To avoide trace events in the browser from showing their
            // JS backtrace, which is very annoying, in my opinion
            MakeConsoleWriter::default().map_trace_level_to(tracing::Level::DEBUG),
        )
        // For some reason, if we don't do this in the browser, we get
        // a runtime error.
        .without_time();

    let log_mem_write = fmt::layer()
        .with_line_number(true)
        .with_writer(move || string_writer.clone())
        .with_ansi(false)
        .without_time()
        .with_level(true)
        .pretty()
        .with_filter(LevelFilter::DEBUG);

    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(log_mem_write);

    set_global_default(subscriber).expect("Failed to set global default subscriber");

    let endpoint = Endpoint {
        main_endpoint: std::borrow::Cow::Borrowed("wss://tvmate.deepgaurav.com"),
    };
    let log_provider = LogProvider { logs };

    let fullsreen_provider = FullScreenProvider {
        exit_fullscreen: Callback::new(move |_| {
            leptos::spawn_local(async move {
                info!("Exit fullscreen");
                let response: Option<String> =
                    tauri_sys::core::invoke("exit_fullscreen", Option::<String>::None).await;
            });
            true
        }),
        fullscreen: Callback::new(move |_| {
            leptos::spawn_local(async move {
                info!("Enter fullscreen");
                let response: Option<String> =
                    tauri_sys::core::invoke("fullscreen", Option::<String>::None).await;
            });
            true
        }),
        share_url: Callback::new(move |request: ShareRequest| {
            leptos::spawn_local(async move {
                #[derive(Serialize, Deserialize)]
                struct Payload {
                    payload: ShareRequest,
                };

                let response: Option<String> =
                    tauri_sys::core::invoke("share", Payload { payload: request }).await;
            });
        }),
    };
    mount_to_body(|| {
        provide_context(fullsreen_provider);
        provide_context(log_provider);
        provide_context(endpoint);
        view! { <App /> }
    })
}
