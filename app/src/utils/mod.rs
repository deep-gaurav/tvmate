use leptos::{document, window};
use tracing::info;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    js_sys::{encode_uri_component, Array},
    Blob, BlobPropertyBag, HtmlElement, Url,
};

pub fn download_logs(logs: String) -> Result<(), JsValue> {
    let blob_data = Array::of1(&JsValue::from_str(&logs));
    let blob = Blob::new_with_blob_sequence_and_options(&blob_data, &{
        let prop = BlobPropertyBag::new();
        prop.set_type("text/plain");
        prop
    })?;
    let href = Url::create_object_url_with_blob(&blob)?;

    info!("Downloading logs");
    let el = document().create_element("a")?;
    el.set_attribute("href", &href)?;
    el.set_attribute("download", "tvmate_logs.log");
    let body = document().body().ok_or(JsValue::from_str("no body"))?;
    body.append_child(el.as_ref())?;
    let html_el: &HtmlElement = el.dyn_ref().ok_or(JsValue::from_str("el not html"))?;
    html_el.click();
    body.remove_child(el.as_ref());
    Ok(())
}
