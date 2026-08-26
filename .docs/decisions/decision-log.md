# Decision Log

Key architecture and design decisions with rationale.

## Deployment & Security

### Docker on Linux LAN (2026-08-20)

**Decision**: Target Docker deployment on Linux, LAN-only access, no authentication.

**Context**: User will run Perun in Docker container on a separate Linux machine, accessible only via local network, never exposed to WAN.

**Rationale**:
- LAN provides trust boundary - no need for auth overhead
- Docker enables simple deployment via public image
- Linux audio device access needed for playback
- Configurable bind address allows flexibility

**Tradeoffs**:
- No auth = must rely on network isolation
- Risk if LAN assumptions violated (accidental WAN exposure)
- Future auth can be added via middleware if deployment changes

**Implementation**:
- Bind address configurable via `PERUN_BIND` env var
- Persistent data directory implemented through `resolve_data_dir()`: use
  `PERUN_DATA_DIR` unchanged when set; otherwise try `/var/lib/perun` and fall
  back to `./soundbank` if creation fails
- Volume mount for `/var/lib/perun` persistent storage
- Security warnings in deployment docs

---

### Environment configuration with defaults (2026-08-22)

**Decision**: Use environment variables as Perun's runtime configuration
surface, with built-in defaults when values are not supplied.

**Context**: The service is intended to run from a Docker image where operators
should be able to choose limits at container start time without editing files in
the image.

**Rationale**:
- Docker and Compose handle environment variables naturally
- Defaults keep local development and simple deployments zero-config
- Invalid numeric values fail startup instead of silently weakening limits
- Avoids adding a `.conf` format before configuration becomes complex

**Implementation**:
```text
PERUN_BIND=0.0.0.0:3030
PERUN_MAX_UPLOAD_MB=10
PERUN_MAX_TITLE_CHARS=100
PERUN_DATA_DIR=/var/lib/perun
```

`PERUN_DATA_DIR` controls the persistent catalog and uploaded sound files.
The Linux default is `/var/lib/perun`; if that directory cannot be created,
Perun falls back to `./soundbank` and logs which directory is active at
startup. Uploaded files live under `<data_dir>/sounds/` and metadata is stored
at `<data_dir>/digest.json`.

---

## API Design

### Query-style with `?id=` or `?title=` (2026-08-20)

**Decision**: Use query parameters (`?id=1` or `?title=doorbell`) for sound lookup, requiring exactly one.

**Context**: Sounds can be identified by numeric `id` or string `title`, both unique. Operations: play, delete, rename.

**Rationale**:
- Both are unique identifiers - either should work
- Query style is simple and explicit
- "Exactly one" validation prevents ambiguity
- Avoids REST path complexity for small API

**Tradeoffs**:
- Not purely RESTful (`/api/sounds/1` would be more REST)
- Slightly more verbose than path params
- Consistent pattern across all operations

**Implementation**:
```
GET    /api/sounds/play?id=1
DELETE /api/sounds/delete?title=doorbell
PATCH  /api/sounds/rename?id=1  (body: {"title": "new"})
```

---

## Data Model

### Duration as formatted string `"m:ss"` (earlier session)

**Decision**: Store `duration` as pre-formatted string (e.g. `"1:05"`) in `SoundMeta` and `digest.json`.

**Context**: Audio files have duration in seconds. Needs to be displayed to clients.

**Rationale**:
- Simplifies display - no formatting needed per-request
- Single source of formatted value
- Acceptable for notification sounds (no complex duration math needed)

**Tradeoffs**:
- ✅ Simple display, consistent formatting
- ❌ Cannot sort/filter by numeric duration without parsing
- ❌ Re-formatting requires parse → modify → format

**Alternative considered**: Store `u64` seconds, format on output.  
**Why rejected**: User explicitly chose display-ready storage.

---

### Sequential numeric IDs (earlier session)

**Decision**: Use sequential `u64` IDs (1, 2, 3, ...) instead of UUIDs.

**Context**: Each sound needs unique identifier for API lookups.

**Rationale**:
- Small, readable, easy to work with in URLs
- Simple to generate (`max + 1`)
- No external dependencies

**Tradeoffs**:
- ✅ Human-friendly, minimal bytes
- ❌ IDs can be reused after deletion (current `max + 1` logic)
- ❌ Reveals approximate catalog size

**Known issue**: After deleting highest-id sound, next upload reuses that id. Solution: persist monotonic `next_id` counter (planned for Phase 4).

---

### Resilient data directory fallback when `/var/lib/perun` is unwritable

**Decision**: When `PERUN_DATA_DIR` is unset, try `/var/lib/perun`; if that
fails, fall back to local `./soundbank` and print the chosen path at startup.

**Context**: Plain `cargo run` should work on macOS and other environments where
`/var/lib/perun` is not writable, without changing Docker/root behavior.

**Rationale**:
- Keeps the Docker/Linux default path unchanged.
- Gives developers a safe local fallback when the default path cannot be used.
- Makes the selected directory obvious in logs.

**Implementation**: `resolve_data_dir()` tries `create_dir_all("/var/lib/perun")`,
falls back to `./soundbank` with explanatory `eprintln!` lines, and
`from_env()` prints `perun: data dir: {path}`.

### Both `id` and `title` are unique (2026-08-20)

**Decision**: Enforce uniqueness on both `id` (always) and `title` (duplicate check in `register`).

**Context**: Users reference sounds by either id or title.

**Rationale**:
- `id` naturally unique (sequential assignment)
- `title` uniqueness prevents confusion (which "doorbell"?)
- Both can serve as lookup key

**Tradeoffs**:
- ✅ Unambiguous lookups by either identifier
- ❌ Cannot have multiple sounds with same title
- ❌ Renaming requires duplicate title check

**Implementation**: `catalog::register` checks for existing title before insert. Future: add title index (`HashMap<String, u64>`) for O(1) lookups.

---

### Consistent API error type with JSON bodies (2026-08-23)

**Decision**: Use a shared `ApiError` enum for HTTP error mapping across API handlers, with JSON `{"error":"..."}` bodies.

**Context**: The sounds handlers had been returning tuple responses plus helper response functions. The API now needs a single error path for bad input, missing resources, conflicts, oversized payloads, and unexpected failures.

**Rationale**:
- Keeps handler signatures simpler
- Makes status-to-body mapping consistent
- Preserves human-readable error messages
- Avoids per-handler helper response functions

**Implementation**: `src/api/error.rs` defines `BadRequest`, `NotFound`, `Conflict`, `PayloadTooLarge`, and `Internal`; it implements `IntoResponse`, and sound handlers now return `Result<(StatusCode, String), ApiError>`.

---

## Performance

### In-memory catalog, no disk reads per request (earlier session)

**Decision**: Load `digest.json` at startup, keep `Store` (in-memory `HashMap`) as source of truth.

**Context**: Catalog reads happen frequently (every play/lookup), writes are rare (upload/delete/rename).

**Rationale**:
- Lookups are instant (no disk I/O per request)
- Writes update both memory and disk atomically
- Fits "instant response" performance goal

**Tradeoffs**:
- ✅ Fast reads, no I/O latency
- ❌ Memory usage scales with catalog size
- ❌ Startup time includes JSON parse

**Implementation**: `new_store()` calls `load_map()`, handlers lock `Arc<Mutex<HashMap>>` for reads/writes.

---

### No sound preloading for now (2026-08-20)

**Decision**: Do not preload audio buffers into memory at startup.

**Context**: For lowest latency, could decode all sounds and keep in RAM.

**Rationale**:
- Not needed yet - premature optimization
- Memory cost grows with catalog
- Can add later if latency testing shows need

**Tradeoffs**:
- ✅ Lower memory usage
- ✅ Simpler initial implementation
- ❌ Playback adds decode step (mitigated by `rodio` streaming)

**Future**: Add optional preloading in Phase 4 if benchmarks justify it.

---

## Audio

### `rodio` over `cpal` for playback (2026-08-20)

**Decision**: Use `rodio` library for audio playback.

**Context**: Need to play MP3 files on Linux. Options: low-level `cpal` or higher-level `rodio`.

**Rationale**:
- `rodio` built on `cpal`, handles decoding/streaming
- Simpler API for basic playback use case
- Good enough for notification sounds
- Can move to `cpal` later if more control needed

**Tradeoffs**:
- ✅ Simple, handles formats, background thread
- ❌ Less control than raw `cpal`

**Implementation**: Actual playback now uses `src/audio/mod.rs` + `src/audio/worker.rs` with a bounded `tokio::sync::mpsc` queue (size 16), `PlaybackCommand { title, path }`, one rodio sink per command, and `spawn_blocking` playback tasks. The worker logs file/decode/device errors and continues draining the queue. `rodio 0.20` was added with the MP3 feature enabled.

### Keep the output stream alive during playback

**Decision**: Bind both tuple elements from `OutputStream::try_default()` in `src/audio/worker.rs` so the `OutputStream` stays alive for the full playback.

**Context**: Dropping the stream immediately made `Sink::try_new(&handle)` fail with `PlayError::NoDevice` even on valid devices.

**Rationale**:
- The audio device should stay open for the life of the playback task.
- The worker should fail only for genuine device/decode issues.
- Preserving the real error text makes failures diagnosable.

**Implementation**: `attempt_play()` now binds `let (_stream, handle)` and maps errors with `format!("{e}")`.

### Initialize tracing subscriber in `main.rs`

**Decision**: Install `tracing_subscriber` at startup with `EnvFilter`, using `RUST_LOG` and defaulting to `info`.

**Context**: Playback success/failure logs were present but invisible without a subscriber.

**Rationale**:
- Keeps logs available during local runs and in Docker.
- Allows environment-based verbosity control.
- Makes the playback smoke test observable.

**Implementation**: `src/main.rs` now initializes tracing before serving requests.

### Wait for sink completion with `sleep_until_end()`

**Decision**: In `src/audio/worker.rs`, wait for each sink with `sleep_until_end()` instead of treating `sink.empty()` as a blocking completion check.

**Context**: `rodio 0.20` `empty()` is a non-blocking boolean query. The earlier code treated it like a wait primitive, which caused the sink to be dropped almost immediately and stopped playback before samples reached the device.

**Rationale**:
- Keeps playback alive for the full duration of the sound.
- Makes the `played sound` log line correspond to real completion time.
- Preserves the current `202 Accepted` HTTP behavior while fixing the worker timing.

**Implementation**: `attempt_play()` now blocks inside `spawn_blocking` until the audio finishes; the temporary `examples/timetest.rs` experiment and the `examples/` dir were removed after verification.

**Verification**: `cargo fmt --check`, `cargo test` (16 tests), `cargo check`, and a live smoke test on `127.0.0.1:3061` showed the `played sound` log ~11s after the request for the 10s file.

---

## Code Organization

### Feature-based API structure (earlier session)

**Decision**: Organize `api/` by feature (vertical slices) not by layer.

**Context**: Could split into `routes.rs`, `handlers.rs`, `model` (horizontal) or feature folders (vertical).

**Rationale**:
- Adding new feature = new folder + one `.nest()` line
- All related code co-located
- Nothing existing needs to grow
- Avoids large shared files

**Implementation**: `api/sounds/` owns its router + handlers. Future: `api/websocket/` etc.

---

### Catalog split by responsibility (2026-08-22)

**Decision**: Split `src/catalog/mod.rs` into focused catalog submodules while
keeping `catalog::...` as the public API used by the rest of the app.

**Context**: The catalog had grown to include the data model, errors, shared
store type, lookup/mutation logic, and JSON persistence in one file. The code
was still small, but the split clarified boundaries before adding configuration,
tests, and upload hardening.

**Rationale**:
- `mod.rs` remains a small facade with public re-exports
- `model.rs` owns only `SoundMeta`
- `errors.rs` owns only catalog-specific error enums
- `persistence.rs` owns configured data directory setup, `<data_dir>/sounds/`
  creation, and digest JSON I/O
- `store.rs` owns in-memory lookup, listing, registration, rename, removal, and
  rollback-aware mutation behavior

**Implementation**: External callers still use `catalog::init()`,
`catalog::new_store()`, `catalog::find()`, `catalog::register()`, and the other
catalog facade exports. Behavior and API routes are unchanged.

---

### Persistent application data directory (2026-08-25)

**Decision**: Store Perun's persistent catalog and uploaded audio under
`/var/lib/perun` by default, using `resolve_data_dir()` to fall back to
`./soundbank` when the default directory cannot be created.

**Rationale**:
- Follows Linux filesystem conventions for application-owned persistent data.
- Keeps uploaded sounds separate from application code and the Docker image.
- Works naturally with a Docker volume mounted at `/var/lib/perun`.
- Avoids `/tmp`, whose contents may be removed during reboot or cleanup.

**Layout**:
```text
/var/lib/perun/
├── digest.json
└── sounds/
```

Local development can use the fallback automatically or set
`PERUN_DATA_DIR=./soundbank` explicitly.

---

## Operations

### No app-owned file logging (2026-08-21)

**Decision**: Remove Perun-managed log files and retention.

**Context**: The earlier implementation wrote application logs to a local file.
The project targets Docker/Linux deployment, where stdout/stderr capture and
retention are normally owned by the container runtime or host supervisor.

**Rationale**:
- Keeps the service smaller and easier to operate in containers
- Avoids project-specific runtime log directories
- Lets Docker/systemd/host tooling handle persistence consistently

**Implementation**: Removed `src/logging.rs`, the runtime log mirror, and the
file-sink dependencies. `tracing` and `tracing-subscriber` remain available for
a future stdout-only subscriber.

---

### Testing update

- `src/catalog/store.rs` now has unit tests for register/find persistence,
  duplicate title rejection, invalid selector validation, sorted digest loading,
  and rename/remove rollback.
- `src/api/mod.rs` now has an async `health_returns_ok` unit test, and
  `src/api/sounds/handlers.rs` now has feature-local handler tests for library
  listing sorted by id, play invalid selector, play not found, play existing
  sound, rename empty title, rename duplicate title with unchanged metadata,
  delete missing sound, delete removing metadata/file, and
  `play_returns_accepted_and_enqueues`.
- Verification passed: `cargo fmt -- --check`, `cargo test` (16 tests), and
  `cargo check` all succeeded.
- Next work is Docker packaging.

### Blocking filesystem hardening

**Decision**: Use `tokio::task::spawn_blocking` for write-path catalog mutations
and audio file removal.

**Rationale**: Keeps async handlers from doing blocking disk work while
preserving current HTTP behavior.

**Implementation**:
- `delete()` wraps catalog mutation + audio file removal in `spawn_blocking`
- missing file removal ignores `NotFound`; join failures map to `500`
- `rename()` wraps catalog rename in `spawn_blocking`
- `upload()` already used `spawn_blocking`
- `list()` stays direct read; `play()` performs lookup and queue enqueue without blocking on decode/playback

## Decisions pending / open questions

### Remaining open questions

- Title index: currently O(n) scan for title lookups. Future: `HashMap<String, u64>` for O(1).
- Monotonic ID counter: currently `max + 1`, reuses deleted IDs. Future: persist `next_id`.
