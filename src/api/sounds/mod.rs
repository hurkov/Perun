mod handlers;
mod types;

use crate::{audio, catalog, config::Settings};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post},
};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct SoundsState {
    pub store: catalog::Store,
    pub settings: Settings,
    pub playback_tx: mpsc::Sender<audio::PlaybackCommand>,
}

pub fn router(
    store: catalog::Store,
    settings: Settings,
    playback_tx: mpsc::Sender<audio::PlaybackCommand>,
) -> Router {
    let max_upload_bytes = settings.max_upload_bytes;
    let state = SoundsState {
        store,
        settings,
        playback_tx,
    };

    Router::new()
        .route(
            "/upload",
            post(handlers::upload).layer(DefaultBodyLimit::max(max_upload_bytes)),
        )
        .route("/library", get(handlers::list))
        .route("/play", get(handlers::play))
        .route("/delete", delete(handlers::delete))
        .route("/rename", patch(handlers::rename))
        .with_state(state)
}
