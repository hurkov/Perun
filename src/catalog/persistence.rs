use std::{collections::HashMap, path::Path};

use super::SoundMeta;

pub fn init(data_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir.join("sounds"))
}

pub fn load_map(data_dir: &Path) -> HashMap<u64, SoundMeta> {
    match std::fs::read_to_string(data_dir.join("digest.json")) {
        Ok(contents) => match serde_json::from_str::<Vec<SoundMeta>>(&contents) {
            Ok(entries) => entries.into_iter().map(|sound| (sound.id, sound)).collect(),
            Err(_) => HashMap::new(),
        },
        Err(_) => HashMap::new(),
    }
}

pub fn save(data_dir: &Path, map: &HashMap<u64, SoundMeta>) -> std::io::Result<()> {
    let entries: Vec<&SoundMeta> = map.values().collect();
    let json = serde_json::to_string(&entries).map_err(std::io::Error::other)?;
    let temporary_path = data_dir.join("digest.json.tmp");
    let digest_path = data_dir.join("digest.json");
    std::fs::write(&temporary_path, json)?;
    std::fs::rename(temporary_path, digest_path)?;
    Ok(())
}
