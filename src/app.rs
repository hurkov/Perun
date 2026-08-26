use crate::{api, audio, catalog, config::Settings};
use axum::Router;
use tokio::sync::mpsc;

/// Bounded queue so a burst of notifications can never grow memory without
/// limit; the newest requests beyond this size get a 503-ish internal error.
pub const PLAYBACK_QUEUE_SIZE: usize = 16;

pub fn router(store: catalog::Store, settings: Settings) -> Router {
    let (playback_tx, playback_rx) = mpsc::channel(PLAYBACK_QUEUE_SIZE);
    tokio::spawn(audio::worker::run(playback_rx));
    api::router(store, settings, playback_tx)
}
