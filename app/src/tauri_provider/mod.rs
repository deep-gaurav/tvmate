use std::{future::Future, pin::Pin};

use leptos::Callback;
use serde::{Deserialize, Serialize};
use web_sys::Element;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareRequest {
    pub url: String,
}

#[derive(Clone)]
pub struct FullScreenProvider {
    pub fullscreen: Callback<Element, bool>,
    pub exit_fullscreen: Callback<(), bool>,
    pub share_url: Callback<ShareRequest, ()>,
}
