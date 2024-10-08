use app::{App, Endpoint};
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    // initializes logging using the `log` crate
    console_error_panic_hook::set_once();

    use tracing_subscriber::fmt;
    use tracing_subscriber_wasm::MakeConsoleWriter;

    fmt()
        .with_writer(
            // To avoide trace events in the browser from showing their
            // JS backtrace, which is very annoying, in my opinion
            MakeConsoleWriter::default().map_trace_level_to(tracing::Level::DEBUG),
        )
        // For some reason, if we don't do this in the browser, we get
        // a runtime error.
        .without_time()
        .init();
    let endpoint = Endpoint {
        main_endpoint: std::borrow::Cow::Borrowed("wss://tvmate.deepgaurav.com"),
    };
    mount_to_body(|| {
        provide_context(endpoint);
        view! { <App /> }
    })
}
