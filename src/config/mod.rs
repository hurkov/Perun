use std::{env, path::PathBuf};

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:3030";
const DEFAULT_DATA_DIR: &str = "/var/lib/perun";
const FALLBACK_DATA_DIR: &str = "./soundbank";
const DEFAULT_MAX_UPLOAD_MB: usize = 10;
const DEFAULT_MAX_TITLE_CHARS: usize = 100;
const BYTES_PER_MIB: usize = 1024 * 1024;

#[derive(Clone)]
pub struct Settings {
    pub bind_address: String,
    pub data_dir: PathBuf,
    pub max_upload_bytes: usize,
    pub max_title_chars: usize,
}

impl Settings {
    pub fn from_env() -> Result<Self, String> {
        let bind_address =
            env::var("PERUN_BIND").unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_string());
        let data_dir = resolve_data_dir()?;
        eprintln!("perun: data dir: {}", data_dir.display());
        let max_upload_mb = read_positive_usize("PERUN_MAX_UPLOAD_MB", DEFAULT_MAX_UPLOAD_MB)?;
        let max_upload_bytes = max_upload_mb
            .checked_mul(BYTES_PER_MIB)
            .ok_or_else(|| "PERUN_MAX_UPLOAD_MB is too large".to_string())?;
        let max_title_chars =
            read_positive_usize("PERUN_MAX_TITLE_CHARS", DEFAULT_MAX_TITLE_CHARS)?;

        Ok(Self {
            bind_address,
            data_dir,
            max_upload_bytes,
            max_title_chars,
        })
    }
}

fn resolve_data_dir() -> Result<PathBuf, String> {
    if let Some(dir) = env::var_os("PERUN_DATA_DIR") {
        if dir.as_os_str().is_empty() {
            return Err("PERUN_DATA_DIR cannot be empty".to_string());
        }
        return Ok(PathBuf::from(dir));
    }

    match std::fs::create_dir_all(DEFAULT_DATA_DIR) {
        Ok(()) => Ok(PathBuf::from(DEFAULT_DATA_DIR)),
        Err(default_error) => {
            let fallback = PathBuf::from(FALLBACK_DATA_DIR);
            std::fs::create_dir_all(&fallback).map_err(|fallback_error| {
                format!(
                    "cannot create data dir {DEFAULT_DATA_DIR} ({default_error}), \
                     fallback {FALLBACK_DATA_DIR} also failed ({fallback_error})"
                )
            })?;
            eprintln!(
                "perun: cannot create {DEFAULT_DATA_DIR} ({default_error}), \
                 falling back to {FALLBACK_DATA_DIR}"
            );
            Ok(fallback)
        }
    }
}

fn read_positive_usize(name: &str, default: usize) -> Result<usize, String> {
    match env::var(name) {
        Ok(value) => {
            let parsed = value
                .parse::<usize>()
                .map_err(|_| format!("{name} must be a positive integer"))?;
            if parsed == 0 {
                return Err(format!("{name} must be greater than zero"));
            }
            Ok(parsed)
        }
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => Err(format!("{name} must be valid UTF-8")),
    }
}
