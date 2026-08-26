use axum::{
    Json,
    extract::{Multipart, Query, State},
    http::StatusCode,
};
use lofty::file::AudioFile;
use lofty::probe::Probe;
use std::io::Cursor;

use super::SoundsState;
use super::types::{RenameBody, SoundQuery};
use crate::api::error::ApiError;
use crate::{
    audio,
    catalog::{self, CatalogError, LookupError, SoundMeta},
};

// POST /api/sounds/upload
pub async fn upload(
    State(state): State<SoundsState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, String), ApiError> {
    let mut title = None;
    let mut bytes = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(_) => return Err(ApiError::BadRequest("invalid multipart request".into())),
        };

        match field.name() {
            Some("title") => match field.text().await {
                Ok(value) => title = Some(value),
                Err(_) => return Err(ApiError::BadRequest("invalid title field".into())),
            },
            Some("file") => match field.bytes().await {
                Ok(value) => bytes = Some(value),
                Err(_) => return Err(ApiError::BadRequest("invalid file field".into())),
            },
            _ => {}
        }
    }

    let (title, bytes) = match (title, bytes) {
        (Some(title), Some(bytes)) if !title.trim().is_empty() && !bytes.is_empty() => {
            (title.trim().to_string(), bytes)
        }
        _ => {
            return Err(ApiError::BadRequest(
                "a non-empty title and file are required".into(),
            ));
        }
    };

    if title.chars().count() > state.settings.max_title_chars {
        return Err(ApiError::BadRequest("title is too long".into()));
    }

    if bytes.len() > state.settings.max_upload_bytes {
        return Err(ApiError::PayloadTooLarge("file is too large".into()));
    }

    let safe_title: String = title
        .chars()
        .filter_map(|character| match character {
            character if character.is_alphanumeric() || character == '-' || character == '_' => {
                Some(character)
            }
            ' ' => Some('-'),
            _ => None,
        })
        .collect();

    if safe_title.is_empty() {
        return Err(ApiError::BadRequest(
            "title must contain a valid filename character".into(),
        ));
    }

    let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(timestamp) => timestamp,
        Err(_) => {
            return Err(ApiError::Internal(
                "system clock is before Unix epoch".into(),
            ));
        }
    };
    let uploaded_date = timestamp.as_secs();
    let file_name = format!("{}_{}.mp3", safe_title, timestamp.as_nanos());
    let path = state.store.sound_file_path(file_name.as_str());
    let path_for_metadata = path.to_string_lossy().into_owned();

    let probe = match Probe::new(Cursor::new(&bytes)).guess_file_type() {
        Ok(probe) => probe,
        Err(_) => return Err(ApiError::BadRequest("invalid audio file".into())),
    };
    let tagged_file = match probe.read() {
        Ok(file) => file,
        Err(_) => return Err(ApiError::BadRequest("invalid audio file".into())),
    };
    let duration = tagged_file.properties().duration().as_secs();

    let store = state.store.clone();
    let write_path = path.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        if std::fs::write(&write_path, &bytes).is_err() {
            return UploadOutcome::WriteFailed;
        }

        match catalog::register(&store, title, duration, path_for_metadata, uploaded_date) {
            Ok(meta) => UploadOutcome::Registered(meta),
            Err(CatalogError::DuplicateTitle) => {
                let _ = std::fs::remove_file(&write_path);
                UploadOutcome::Duplicate
            }
            Err(CatalogError::SaveFailed | CatalogError::NotFound) => {
                let _ = std::fs::remove_file(&write_path);
                UploadOutcome::MetadataSaveFailed
            }
        }
    })
    .await;

    match outcome {
        Ok(UploadOutcome::Registered(meta)) => {
            Ok((StatusCode::OK, format!("uploaded with id {}", meta.id)))
        }
        Ok(UploadOutcome::Duplicate) => Err(ApiError::Conflict(
            "a sound with that title already exists".into(),
        )),
        Ok(UploadOutcome::MetadataSaveFailed) => {
            Err(ApiError::Internal("failed to save sound metadata".into()))
        }
        Ok(UploadOutcome::WriteFailed) | Err(_) => {
            Err(ApiError::Internal("failed to save file".into()))
        }
    }
}

enum UploadOutcome {
    Registered(SoundMeta),
    Duplicate,
    MetadataSaveFailed,
    WriteFailed,
}

// GET /api/sounds/library
pub async fn list(State(state): State<SoundsState>) -> (StatusCode, Json<Vec<SoundMeta>>) {
    (StatusCode::OK, Json(catalog::list(&state.store)))
}

// GET /api/sounds/play
pub async fn play(
    State(state): State<SoundsState>,
    Query(query): Query<SoundQuery>,
) -> Result<(StatusCode, String), ApiError> {
    let sound = match catalog::find(&state.store, query.title.as_deref(), query.id) {
        Ok(sound) => sound,
        Err(LookupError::InvalidSelector) => {
            return Err(ApiError::BadRequest(
                "exactly one of title or id is required".into(),
            ));
        }
        Err(LookupError::NotFound) => {
            return Err(ApiError::NotFound(
                "notification sound doesn't exist".into(),
            ));
        }
    };

    let command = audio::PlaybackCommand {
        title: sound.title,
        path: sound.path,
    };
    match state.playback_tx.try_send(command) {
        Ok(()) => Ok((StatusCode::ACCEPTED, "playback queued".into())),
        Err(tokio::sync::mpsc::error::TrySendError::Full(cmd)) => {
            let title = cmd.title;
            Err(ApiError::Internal(format!(
                "playback queue is full, \"{title}\" was not queued"
            )))
        }
        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
            Err(ApiError::Internal("playback worker is not running".into()))
        }
    }
}

// DELETE /api/sounds/delete
pub async fn delete(
    State(state): State<SoundsState>,
    Query(query): Query<SoundQuery>,
) -> Result<(StatusCode, String), ApiError> {
    let sound = match catalog::find(&state.store, query.title.as_deref(), query.id) {
        Ok(sound) => sound,
        Err(LookupError::InvalidSelector) => {
            return Err(ApiError::BadRequest(
                "exactly one of title or id is required".into(),
            ));
        }
        Err(LookupError::NotFound) => return Err(ApiError::NotFound("sound not found".into())),
    };

    let store = state.store.clone();
    let sound_id = sound.id;
    let result = tokio::task::spawn_blocking(move || match catalog::remove(&store, sound_id) {
        Ok(removed) => {
            if let Err(error) = std::fs::remove_file(&removed.path)
                && error.kind() != std::io::ErrorKind::NotFound
            {}
            Ok(())
        }
        Err(error) => Err(error),
    })
    .await;

    match result {
        Ok(Ok(())) => Ok((StatusCode::OK, "deleted sound successfully".to_string())),
        Ok(Err(CatalogError::NotFound)) => Err(ApiError::NotFound("sound not found".into())),
        Ok(Err(CatalogError::DuplicateTitle | CatalogError::SaveFailed)) => {
            Err(ApiError::Internal("failed to delete sound metadata".into()))
        }
        Err(_) => Err(ApiError::Internal("failed to delete sound metadata".into())),
    }
}

// PATCH /api/sounds/rename
pub async fn rename(
    State(state): State<SoundsState>,
    Query(query): Query<SoundQuery>,
    Json(body): Json<RenameBody>,
) -> Result<(StatusCode, String), ApiError> {
    if body.title.trim().is_empty() {
        return Err(ApiError::BadRequest("new title cannot be empty".into()));
    }

    let sound = match catalog::find(&state.store, query.title.as_deref(), query.id) {
        Ok(sound) => sound,
        Err(LookupError::InvalidSelector) => {
            return Err(ApiError::BadRequest(
                "exactly one of title or id is required".into(),
            ));
        }
        Err(LookupError::NotFound) => return Err(ApiError::NotFound("sound not found".into())),
    };

    let store = state.store.clone();
    let new_title = body.title.clone();
    let sound_id = sound.id;
    let result =
        tokio::task::spawn_blocking(move || catalog::rename(&store, new_title, sound_id)).await;

    match result {
        Ok(Ok(_)) => Ok((StatusCode::OK, "sound renamed successfully".to_string())),
        Ok(Err(CatalogError::DuplicateTitle)) => Err(ApiError::Conflict(
            "a sound with that title already exists".into(),
        )),
        Ok(Err(CatalogError::NotFound)) => Err(ApiError::NotFound("sound not found".into())),
        Ok(Err(CatalogError::SaveFailed)) => {
            Err(ApiError::Internal("failed to save title update".into()))
        }
        Err(_) => Err(ApiError::Internal("failed to save title update".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use axum::response::IntoResponse;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("perun_handlers_test_{name}_{nanos}"))
    }

    fn setup_state(name: &str) -> (SoundsState, PathBuf) {
        let data_dir = unique_dir(name);
        catalog::init(&data_dir).unwrap();
        let store = catalog::new_store(data_dir.clone());
        let settings = Settings {
            bind_address: "127.0.0.1:0".to_string(),
            data_dir: data_dir.clone(),
            max_upload_bytes: 1024 * 1024,
            max_title_chars: 100,
        };
        let (playback_tx, mut playback_rx) = tokio::sync::mpsc::channel(16);
        tokio::spawn(async move { while let Some(_) = playback_rx.recv().await {} });
        (
            SoundsState {
                store,
                settings,
                playback_tx,
            },
            data_dir,
        )
    }

    fn cleanup(data_dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(data_dir);
    }

    async fn handler_response(
        result: Result<(StatusCode, String), ApiError>,
    ) -> (StatusCode, String) {
        let response = match result {
            Ok((status, body)) => (status, body).into_response(),
            Err(error) => error.into_response(),
        };
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[tokio::test]
    async fn list_returns_registered_sounds_sorted_by_id() {
        let (state, data_dir) = setup_state("list_sorted");

        catalog::register(
            &state.store,
            "Beta".to_string(),
            60,
            "beta.mp3".to_string(),
            2,
        )
        .unwrap();
        catalog::register(
            &state.store,
            "Alpha".to_string(),
            60,
            "alpha.mp3".to_string(),
            1,
        )
        .unwrap();

        let (status, Json(sounds)) = list(State(state)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(sounds.len(), 2);
        assert!(sounds[0].id < sounds[1].id);

        cleanup(&data_dir);
    }

    #[tokio::test]
    async fn play_rejects_invalid_selector() {
        let (state, data_dir) = setup_state("play_invalid_selector");

        let (status, body) = handler_response(
            play(
                State(state),
                Query(SoundQuery {
                    title: None,
                    id: None,
                }),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains("exactly one of title or id is required"));

        cleanup(&data_dir);
    }

    #[tokio::test]
    async fn play_returns_not_found_for_missing_sound() {
        let (state, data_dir) = setup_state("play_missing");

        let (status, body) = handler_response(
            play(
                State(state),
                Query(SoundQuery {
                    title: None,
                    id: Some(999),
                }),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("notification sound doesn't exist"));

        cleanup(&data_dir);
    }

    #[tokio::test]
    async fn play_returns_accepted_and_enqueues() {
        let (state, data_dir) = setup_state("play_existing");

        let meta = catalog::register(
            &state.store,
            "Ping".to_string(),
            60,
            "ping.mp3".to_string(),
            1,
        )
        .unwrap();

        let result = play(
            State(state),
            Query(SoundQuery {
                title: None,
                id: Some(meta.id),
            }),
        )
        .await;

        let (status, body) = result.expect("play should succeed");
        assert_eq!(status, StatusCode::ACCEPTED);
        assert!(body.contains("playback queued"));

        cleanup(&data_dir);
    }

    #[tokio::test]
    async fn rename_rejects_empty_title() {
        let (state, data_dir) = setup_state("rename_empty");

        let meta = catalog::register(
            &state.store,
            "Original".to_string(),
            60,
            "original.mp3".to_string(),
            1,
        )
        .unwrap();

        let (status, body) = handler_response(
            rename(
                State(state),
                Query(SoundQuery {
                    title: None,
                    id: Some(meta.id),
                }),
                Json(RenameBody { title: "  ".into() }),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains("new title cannot be empty"));

        cleanup(&data_dir);
    }

    #[tokio::test]
    async fn rename_rejects_duplicate_title() {
        let (state, data_dir) = setup_state("rename_duplicate");

        let first = catalog::register(
            &state.store,
            "First".to_string(),
            60,
            "first.mp3".to_string(),
            1,
        )
        .unwrap();
        catalog::register(
            &state.store,
            "Second".to_string(),
            60,
            "second.mp3".to_string(),
            2,
        )
        .unwrap();

        let (status, body) = handler_response(
            rename(
                State(state.clone()),
                Query(SoundQuery {
                    title: None,
                    id: Some(first.id),
                }),
                Json(RenameBody {
                    title: "Second".to_string(),
                }),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert!(body.contains("a sound with that title already exists"));

        let unchanged = catalog::find(&state.store, None, Some(first.id)).unwrap();
        assert_eq!(unchanged.title, "First");

        cleanup(&data_dir);
    }

    #[tokio::test]
    async fn delete_returns_not_found_for_missing_sound() {
        let (state, data_dir) = setup_state("delete_missing");

        let (status, body) = handler_response(
            delete(
                State(state),
                Query(SoundQuery {
                    title: None,
                    id: Some(999),
                }),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("sound not found"));

        cleanup(&data_dir);
    }

    #[tokio::test]
    async fn delete_removes_metadata_and_file() {
        let (state, data_dir) = setup_state("delete_removes");

        let file_path = state.store.sound_file_path("placeholder.mp3");
        std::fs::write(&file_path, b"placeholder").unwrap();

        let meta = catalog::register(
            &state.store,
            "ToDelete".to_string(),
            60,
            file_path.to_string_lossy().into_owned(),
            1,
        )
        .unwrap();

        let result = delete(
            State(state.clone()),
            Query(SoundQuery {
                title: None,
                id: Some(meta.id),
            }),
        )
        .await;

        let (status, _) = result.expect("delete should succeed");
        assert_eq!(status, StatusCode::OK);
        assert!(matches!(
            catalog::find(&state.store, None, Some(meta.id)),
            Err(LookupError::NotFound)
        ));
        assert!(!file_path.exists());

        cleanup(&data_dir);
    }
}
