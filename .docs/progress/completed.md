# Completed Work

## Recent milestones

### Final playback completion fix ✅

- `src/audio/worker.rs` replaced `sink.empty()` with `sink.sleep_until_end()` inside `attempt_play()`.
- Root cause: `rodio 0.20` `empty()` is a non-blocking bool check, so the previous code dropped the sink almost instantly and stopped playback before any samples reached the device.
- `attempt_play()` now blocks on its `spawn_blocking` thread until the sound fully finishes, so the `played sound` log lines up with the actual playback window.
- Leftover `examples/timetest.rs` and the `examples/` dir were removed.
- Verification passed: `cargo fmt --check`, `cargo test` (16 tests), `cargo check`, plus a live smoke test on `127.0.0.1:3061` where the log landed ~11s after the request for the 10s file.

### Config data-dir fallback ✅

- `resolve_data_dir()` now uses `PERUN_DATA_DIR` unchanged when set.
- When `PERUN_DATA_DIR` is unset, Perun tries to create `/var/lib/perun`; if that fails, it falls back to `./soundbank` with explanatory `eprintln!` lines.
- `from_env()` now prints `perun: data dir: {path}` at startup, and the unused `read_path` helper was removed.
- Verification passed: `cargo fmt --check`, `cargo test` (16 tests), `cargo check`, plus live checks for both paths (`./soundbank` fallback on plain Mac run, `/tmp/perun_cfg_test` when `PERUN_DATA_DIR` was set).

### Playback NoDevice root-cause fix ✅

- `src/audio/worker.rs` now binds both `OutputStream::try_default()` tuple elements (`let (_stream, handle)`) so the stream stays alive for the whole playback.
- Playback error mapping now preserves the underlying rodio error text with `format!("{e}")` instead of an unmapped string.
- `src/main.rs` initializes `tracing_subscriber` with `EnvFilter` (`RUST_LOG`, default `info`) so playback logs are visible.
- Temporary experiment files `examples/play_once.rs`, `examples/play_drop.rs`, and `examples/devices.rs` were removed.
- Verification passed: `cargo fmt -- --check`, `cargo test` (16 tests), `cargo check`, and a live smoke test (`curl play?id=1 -> 202`, then `played sound title=Test2`).

### Playback implementation ✅

- `src/audio/mod.rs` introduces `PlaybackCommand { title, path }`.
- `src/app.rs` creates the bounded `tokio::sync::mpsc` playback queue (`PLAYBACK_QUEUE_SIZE = 16`) and spawns the worker.
- `src/api/sounds/mod.rs` threads the sender into `SoundsState`.
- `GET /api/sounds/play` now enqueues playback and returns `202 Accepted` with `playback queued`.
- Full or closed queue paths return `500`; `400/404` selector handling stays unchanged.
- `src/audio/worker.rs` uses `rodio 0.20` with MP3 support, creates a fresh sink per command, and logs-and-continues on file, decode, or device errors.
- Verification passed: `cargo fmt -- --check`, `cargo test` (16 tests), and `cargo check`.

### Sound catalog foundation ✅

- `POST /api/sounds/upload` - multipart upload with audio validation
- `GET /api/sounds/library` - in-memory catalog listing
- `catalog::register()` - duplicate detection, sequential IDs, atomic save with rollback
- `catalog::save()` - temp file + rename for atomic writes
- `catalog::load_map()` - startup digest loading
- `catalog::remove()` - delete with save+rollback pattern
- `duration` field - stored as formatted `"m:ss"` string
- Initial routes - `GET /api/sounds/play` and `DELETE /api/sounds/delete` registered
- Catalog module split into `model.rs`, `errors.rs`, `persistence.rs`,
  `store.rs`, with `mod.rs` as the public facade

### API handler tests ✅

- `src/api/mod.rs` now has an async `health_returns_ok` unit test.
- `src/api/sounds/handlers.rs` now has feature-local tests for sorted library listing, play invalid selector, play not found, play existing sound, rename empty title, rename duplicate title with unchanged metadata, delete missing sound, and delete removing metadata/file.

### API error handling ✅

- `src/api/error.rs` now centralizes `ApiError` with `BadRequest`, `NotFound`, `Conflict`, `PayloadTooLarge`, and `Internal`.
- API errors return JSON `{"error":"..."}` bodies via `IntoResponse`.
- Sound handlers now return `Result<(StatusCode, String), ApiError>` and the old helper response functions were removed.

### Blocking filesystem hardening ✅

- `delete()` now runs catalog mutation plus audio file removal inside `tokio::task::spawn_blocking`.
- Missing-file removal ignores `NotFound`; join failures map to `500`.
- `rename()` now wraps catalog rename in `spawn_blocking` with unchanged `200/409/404/500` mapping.
- `upload()` already used `spawn_blocking`.
- `list()` stays direct read with lock-only access and no disk writes; `play()` now performs lookup plus queue enqueue only.
- Behavior stayed unchanged; all 16 tests still pass, and `cargo fmt -- --check` plus `cargo check` are clean.

### Catalog store tests ✅

- `src/catalog/store.rs` now covers register/find persistence, duplicate title
  rejection, invalid selector validation, sorted digest loading, rename
  rollback on save failure, and remove rollback on save failure.

### Infrastructure ✅

- Module split: `app.rs`, `audio/`, `api/`, and responsibility-based `catalog/`
- Graceful shutdown via Ctrl-C signal handling
- Feature-based API structure (`api/sounds/` vertical slice)
- Environment settings with defaults for `PERUN_BIND`, `PERUN_MAX_UPLOAD_MB`,
  `PERUN_MAX_TITLE_CHARS`, and `PERUN_DATA_DIR`
- Upload body and file/title limit checks
- Bruno API collection for all currently implemented endpoints

### Documentation ✅

- Migrated to `.docs/` structure
- `ROUTER.md` + `ROUTER.json` navigation
- `progress/` folder with current-focus, next-steps, completed
- `decisions/` folder with decision log
- `technical/` folder with architecture, API, deployment, development, logging policy, and audio playback docs
- Deployment decisions documented (Docker, LAN-only, no auth, configurable port,
  persistent data directory, and audio device access)

## Validation ✅

- `cargo fmt -- --check` passed.
- `cargo test` ran 16 tests and all passed: 7 `catalog::store` tests, 1
  `api::health` test, and 8 `api::sounds::handlers` tests.
- `cargo check` passed.
- Next work: Docker packaging.

## What was decided along the way

See [`../decisions/decision-log.md`](../decisions/decision-log.md) for full context.

Key choices:
- Duration stored as display string `"m:ss"` (tradeoff: simple display vs numeric operations)
- Sequential numeric IDs instead of UUIDs (tradeoff: simple/readable vs potential reuse after delete)
- Both `id` and `title` unique and usable for lookups
- Query-style endpoints (`?id=` or `?title=`)
- LAN-only deployment, no authentication (network is trust boundary)
- Configurable bind address for Docker flexibility
