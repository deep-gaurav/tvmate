use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_tvmate);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<Tvmate<R>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin("com.plugin.tvmate", "ExamplePlugin")?;
    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_tvmate)?;
    Ok(Tvmate(handle))
}

/// Access to the tvmate APIs.
pub struct Tvmate<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Tvmate<R> {
    pub fn is_fullscreen(&self) -> crate::Result<FullScreenResponse> {
        self.0
            .run_mobile_plugin("isFullscreen", EmptyRequest)
            .map_err(Into::into)
    }

    pub fn exit_fullscreen(&self) -> crate::Result<FullScreenResponse> {
        self.0
            .run_mobile_plugin("exitFullscreen", EmptyRequest)
            .map_err(Into::into)
    }

    pub fn fullscreen(&self, payload: FullScreenRequest) -> crate::Result<FullScreenResponse> {
        self.0
            .run_mobile_plugin("fullscreen", payload)
            .map_err(Into::into)
    }
}
