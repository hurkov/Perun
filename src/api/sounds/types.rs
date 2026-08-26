use serde::Deserialize;

/// Query parameters for sound lookup operations (play, delete, rename).
/// Exactly one of `id` or `title` must be provided.
#[derive(Deserialize)]
pub struct SoundQuery {
    pub title: Option<String>,
    pub id: Option<u64>,
}

/// Request body for renaming a sound's title.
#[derive(Deserialize)]
pub struct RenameBody {
    pub title: String,
}
