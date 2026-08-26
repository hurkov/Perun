use super::persistence;
use super::{CatalogError, LookupError, SoundMeta};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone)]
pub struct Store {
    data_dir: Arc<PathBuf>,
    sounds: Arc<Mutex<HashMap<u64, SoundMeta>>>,
}

impl Store {
    pub fn sound_file_path(&self, file_name: impl AsRef<Path>) -> PathBuf {
        self.data_dir.join("sounds").join(file_name)
    }
}

pub fn new_store(data_dir: PathBuf) -> Store {
    let sounds = persistence::load_map(&data_dir);
    Store {
        data_dir: Arc::new(data_dir),
        sounds: Arc::new(Mutex::new(sounds)),
    }
}

fn lock_store(store: &Store) -> MutexGuard<'_, HashMap<u64, SoundMeta>> {
    match store.sounds.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub fn list(store: &Store) -> Vec<SoundMeta> {
    let map = lock_store(store);
    let mut sounds: Vec<SoundMeta> = map.values().cloned().collect();
    sounds.sort_by_key(|sound| sound.id);
    sounds
}

pub fn find(store: &Store, title: Option<&str>, id: Option<u64>) -> Result<SoundMeta, LookupError> {
    let map = lock_store(store);
    match (title, id) {
        (Some(title), None) => map
            .values()
            .find(|sound| sound.title == title)
            .cloned()
            .ok_or(LookupError::NotFound),
        (None, Some(id)) => map.get(&id).cloned().ok_or(LookupError::NotFound),
        _ => Err(LookupError::InvalidSelector),
    }
}

pub fn remove(store: &Store, id: u64) -> Result<SoundMeta, CatalogError> {
    let mut map = lock_store(store);
    let removed = map.remove(&id).ok_or(CatalogError::NotFound)?;

    if persistence::save(&store.data_dir, &map).is_err() {
        map.insert(id, removed);
        return Err(CatalogError::SaveFailed);
    }

    Ok(removed)
}

pub fn rename(store: &Store, new_title: String, id: u64) -> Result<SoundMeta, CatalogError> {
    let mut map = lock_store(store);

    if map
        .values()
        .any(|sound| sound.title == new_title && sound.id != id)
    {
        return Err(CatalogError::DuplicateTitle);
    }

    let sound = map.get_mut(&id).ok_or(CatalogError::NotFound)?;
    let old_title = std::mem::replace(&mut sound.title, new_title);
    let updated = sound.clone();

    if persistence::save(&store.data_dir, &map).is_err() {
        if let Some(sound) = map.get_mut(&id) {
            sound.title = old_title;
        }
        return Err(CatalogError::SaveFailed);
    }

    Ok(updated)
}

pub fn register(
    store: &Store,
    title: String,
    duration: u64,
    path: String,
    uploaded_date: u64,
) -> Result<SoundMeta, CatalogError> {
    let mut map = lock_store(store);

    if map.values().any(|sound| sound.title == title) {
        return Err(CatalogError::DuplicateTitle);
    }

    let id = map.keys().max().map_or(1, |max_id| max_id + 1);
    let meta = SoundMeta {
        id,
        title,
        duration: format_duration(duration),
        path,
        uploaded_date,
    };
    map.insert(id, meta.clone());

    if persistence::save(&store.data_dir, &map).is_err() {
        map.remove(&id);
        return Err(CatalogError::SaveFailed);
    }

    Ok(meta)
}

fn format_duration(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("perun_catalog_test_{name}_{nanos}"))
    }

    fn sample_meta(id: u64, title: &str) -> SoundMeta {
        SoundMeta {
            id,
            title: title.to_string(),
            duration: "1:00".to_string(),
            path: format!("{title}.mp3"),
            uploaded_date: id,
        }
    }

    #[test]
    fn register_persists_and_can_be_found_by_id_and_title() {
        let data_dir = unique_dir("register");
        persistence::init(&data_dir).unwrap();
        let store = new_store(data_dir.clone());

        let meta = register(
            &store,
            "Song A".to_string(),
            125,
            "song-a.mp3".to_string(),
            1000,
        )
        .unwrap();

        assert_eq!(meta.title, "Song A");
        assert_eq!(meta.duration, "2:05");

        let by_id = find(&store, None, Some(meta.id)).unwrap();
        assert_eq!(by_id.title, "Song A");

        let by_title = find(&store, Some("Song A"), None).unwrap();
        assert_eq!(by_title.id, meta.id);

        let saved_contents = std::fs::read_to_string(data_dir.join("digest.json")).unwrap();
        assert!(saved_contents.contains("Song A"));

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn register_rejects_duplicate_title_and_keeps_store_length() {
        let data_dir = unique_dir("dup_title");
        persistence::init(&data_dir).unwrap();
        let store = new_store(data_dir.clone());

        register(&store, "Same Title".to_string(), 60, "a.mp3".to_string(), 1).unwrap();
        let result = register(&store, "Same Title".to_string(), 90, "b.mp3".to_string(), 2);

        assert!(matches!(result, Err(CatalogError::DuplicateTitle)));
        assert_eq!(list(&store).len(), 1);

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn find_requires_exactly_one_selector() {
        let data_dir = unique_dir("selector");
        persistence::init(&data_dir).unwrap();
        let store = new_store(data_dir.clone());

        assert!(matches!(
            find(&store, None, None),
            Err(LookupError::InvalidSelector)
        ));
        assert!(matches!(
            find(&store, Some("x"), Some(1)),
            Err(LookupError::InvalidSelector)
        ));

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn new_store_loads_existing_digest_and_list_sorts_by_id() {
        let data_dir = unique_dir("load_existing");
        persistence::init(&data_dir).unwrap();

        let mut map = HashMap::new();
        map.insert(2, sample_meta(2, "Second"));
        map.insert(1, sample_meta(1, "First"));
        persistence::save(&data_dir, &map).unwrap();

        let store = new_store(data_dir.clone());
        let sounds = list(&store);

        assert_eq!(sounds.len(), 2);
        assert_eq!(sounds[0].id, 1);
        assert_eq!(sounds[1].id, 2);

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn rename_rejects_duplicate_title() {
        let data_dir = unique_dir("rename_dup");
        persistence::init(&data_dir).unwrap();
        let store = new_store(data_dir.clone());

        let first = register(&store, "First".to_string(), 60, "a.mp3".to_string(), 1).unwrap();
        register(&store, "Second".to_string(), 60, "b.mp3".to_string(), 2).unwrap();

        let result = rename(&store, "Second".to_string(), first.id);

        assert!(matches!(result, Err(CatalogError::DuplicateTitle)));
        let unchanged = find(&store, None, Some(first.id)).unwrap();
        assert_eq!(unchanged.title, "First");

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn rename_rolls_back_when_save_fails() {
        let data_dir = unique_dir("rename_rollback");
        std::fs::write(&data_dir, b"not a directory").unwrap();

        let mut map = HashMap::new();
        map.insert(1, sample_meta(1, "Original"));
        let store = Store {
            data_dir: Arc::new(data_dir.clone()),
            sounds: Arc::new(Mutex::new(map)),
        };

        let result = rename(&store, "Updated".to_string(), 1);

        assert!(matches!(result, Err(CatalogError::SaveFailed)));
        let sound = find(&store, None, Some(1)).unwrap();
        assert_eq!(sound.title, "Original");

        let _ = std::fs::remove_file(&data_dir);
    }

    #[test]
    fn remove_rolls_back_when_save_fails() {
        let data_dir = unique_dir("remove_rollback");
        std::fs::write(&data_dir, b"not a directory").unwrap();

        let mut map = HashMap::new();
        map.insert(1, sample_meta(1, "Original"));
        let store = Store {
            data_dir: Arc::new(data_dir.clone()),
            sounds: Arc::new(Mutex::new(map)),
        };

        let result = remove(&store, 1);

        assert!(matches!(result, Err(CatalogError::SaveFailed)));
        let sound = find(&store, None, Some(1)).unwrap();
        assert_eq!(sound.title, "Original");

        let _ = std::fs::remove_file(&data_dir);
    }
}
