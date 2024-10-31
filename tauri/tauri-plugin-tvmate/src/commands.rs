use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::Result;
use crate::TvmateExt;

#[command]
pub(crate) async fn fullscreen<R: Runtime>(
    app: AppHandle<R>,
    payload: FullScreenRequest,
) -> Result<FullScreenResponse> {
    app.tvmate().fullscreen(payload)
}

#[command]
pub(crate) async fn share_url<R: Runtime>(app: AppHandle<R>, payload: ShareRequest) -> Result<()> {
    app.tvmate().share_url(payload)
}
