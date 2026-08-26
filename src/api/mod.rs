pub mod error;
pub mod help;
pub mod sounds;

use crate::{audio, catalog, config::Settings};
use axum::{Router, http::StatusCode, routing::get};
use tokio::sync::mpsc;

pub fn router(
    store: catalog::Store,
    settings: Settings,
    playback_tx: mpsc::Sender<audio::PlaybackCommand>,
) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/help", get(help::handle))
        .nest("/sounds", sounds::router(store, settings, playback_tx))
}

async fn health() -> StatusCode {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn health_returns_ok() {
        assert_eq!(health().await, StatusCode::OK);
    }

    #[tokio::test]
    async fn help_returns_documentation_text() {
        let response = help::handle().await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/plain; charset=utf-8"
        );
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body.contains("play"));
    }
}
