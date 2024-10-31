use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Tvmate<R>> {
    Ok(Tvmate(app.clone()))
}

/// Access to the tvmate APIs.
pub struct Tvmate<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Tvmate<R> {
    pub fn is_fullscreen(&self) -> crate::Result<FullScreenResponse> {
        unimplemented!("Full screen not implemented in desktop")
    }

    pub fn exit_fullscreen(&self) -> crate::Result<FullScreenResponse> {
        unimplemented!("Full screen not implemented in desktop")
    }

    pub fn fullscreen(&self, payload: FullScreenRequest) -> crate::Result<FullScreenResponse> {
        unimplemented!("Full screen not implemented in desktop")
    }

    pub fn share_url(&self, payload: ShareRequest) -> crate::Result<()> {
        unimplemented!("Share not implemented in desktop")
    }
}
