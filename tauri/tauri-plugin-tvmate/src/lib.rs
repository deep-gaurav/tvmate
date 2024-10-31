use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Tvmate;
#[cfg(mobile)]
use mobile::Tvmate;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the tvmate APIs.
pub trait TvmateExt<R: Runtime> {
    fn tvmate(&self) -> &Tvmate<R>;
}

impl<R: Runtime, T: Manager<R>> crate::TvmateExt<R> for T {
    fn tvmate(&self) -> &Tvmate<R> {
        self.state::<Tvmate<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("tvmate")
        .invoke_handler(tauri::generate_handler![
            commands::fullscreen,
            commands::share_url
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let tvmate = mobile::init(app, api)?;
            #[cfg(desktop)]
            let tvmate = desktop::init(app, api)?;
            app.manage(tvmate);
            Ok(())
        })
        .build()
}
