use std::{future::Future, pin::Pin};

use leptos::Callback;

#[derive(Clone)]
pub struct FullScreenProvider {
    pub fullscreen: Callback<(), bool>,
    pub exit_fullscreen: Callback<(), bool>,
}
