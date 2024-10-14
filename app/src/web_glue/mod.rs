use wasm_bindgen::prelude::*;
use web_sys::{js_sys, HtmlMediaElement, MediaStream};

#[wasm_bindgen]
extern "C" {

    #[wasm_bindgen (extends = HtmlMediaElement, extends = js_sys::Object, js_name = HTMLMediaElement, typescript_type = "HTMLMediaElement")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub type HtmlMediaElement2;

    # [wasm_bindgen (catch , method , structural , js_class = "HTMLMediaElement" , js_name = captureStream)]
    pub fn capture_stream(this: &HtmlMediaElement2) -> Result<MediaStream, JsValue>;
}
