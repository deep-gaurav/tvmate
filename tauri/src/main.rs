use app::{App, Endpoint};
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    let endpoint = Endpoint {
        main_endpoint: std::borrow::Cow::Borrowed("wss://tvmate.deepgaurav.com"),
    };
    mount_to_body(|| {
        provide_context(endpoint);
        view! {
            <App/>
        }
    })
}
