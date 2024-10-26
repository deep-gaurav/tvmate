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
