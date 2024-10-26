use std::io::Write;

use leptos::{document, window, StoredValue};
use tracing::info;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    js_sys::{encode_uri_component, Array, Date},
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

#[derive(Clone)]
pub struct StringWriter {
    pub log_buffer: StoredValue<String>,
}

impl Write for StringWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(s) = String::from_utf8(buf.to_vec()) {
            let date = Date::new_0();
            self.log_buffer.update_value(|buffer| {
                buffer.push_str(&format!("{}: {}", date.to_string(), &s));
            });
            Ok(buf.len())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid UTF-8",
            ))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
