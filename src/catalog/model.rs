use serde::{Deserialize, Serialize};
#[derive(Clone, Deserialize, Serialize)]
pub struct SoundMeta {
    pub id: u64,
    pub title: String,
    pub duration: String,
    pub path: String,
    pub uploaded_date: u64,
}
