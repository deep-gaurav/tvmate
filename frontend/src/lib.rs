use std::io::Write;

use app::*;
use leptos::*;
use tracing::{level_filters::LevelFilter, subscriber::set_global_default};
use tracing_subscriber::{fmt::format::Writer, layer::SubscriberExt, Layer};
use utils::StringWriter;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::js_sys::Date;

#[wasm_bindgen]
pub fn hydrate() {
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
        main_endpoint: std::borrow::Cow::Borrowed(""),
    };
    let log_provider = LogProvider { logs };

    leptos::mount_to_body(move || {
        provide_context(endpoint);
        provide_context(log_provider);
        view! { <App /> }
    });
}
