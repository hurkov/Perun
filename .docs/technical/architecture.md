# Architecture

## Current source tree

```text
src/
├── main.rs                 # startup, catalog initialization, server lifecycle
├── app.rs                  # creates playback queue, spawns worker, mounts /api
├── audio/
│   ├── mod.rs              # playback command type and worker re-export
│   └── worker.rs           # bounded queue consumer and rodio playback loop
├── config/
│   └── mod.rs              # environment-backed runtime settings with data-dir fallback
├── catalog/
│   ├── mod.rs              # catalog facade and public re-exports
│   ├── errors.rs           # catalog and lookup error enums
│   ├── model.rs            # SoundMeta data model
│   ├── persistence.rs      # configured data directory and digest JSON persistence
│   └── store.rs            # in-memory store, lookup, mutation logic, and unit tests
└── api/
    ├── error.rs            # shared ApiError enum and IntoResponse mapping
    ├── mod.rs              # API composition, health route, and health test
    └── sounds/
        ├── mod.rs          # sound feature router
        ├── types.rs        # SoundQuery and RenameBody
        └── handlers.rs     # upload/list/play/delete/rename handlers and feature-local tests
```

Empty future directories are intentionally not present. Add a module only when
it owns real behavior.

## Responsibilities

- `main.rs` loads settings from environment variables, initializes the catalog,
  builds the app, binds the configured listener, and handles graceful Ctrl-C
  shutdown.
- `app.rs` creates the bounded playback queue, spawns the worker, and mounts
  `api::router()` at `/api`.
- `audio/mod.rs` defines the playback command type and re-exports the worker.
- `audio/worker.rs` drains the queue and performs rodio playback on blocking
  tasks, using `sleep_until_end()` so each sound runs to completion before the
  worker moves on.
- `config/mod.rs` owns runtime settings with Docker-friendly environment
  overrides and defaults.
- `api/error.rs` owns the shared `ApiError` enum and JSON HTTP error mapping.
- `api/mod.rs` composes feature routers and owns `/api/health` and its unit test.
- `api/sounds/` owns HTTP transport types, routes, validation, success
  responses, upload body-limit wiring, playback queue handoff, and
  feature-local tests.
- `catalog/mod.rs` exposes the catalog public API while keeping the internal
  module layout private.
- `catalog/model.rs` owns `SoundMeta`.
- `catalog/errors.rs` owns catalog and lookup error enums.
- `catalog/persistence.rs` owns the configured data directory setup,
  `<data_dir>/sounds/` creation, and digest JSON I/O.
- `catalog/store.rs` owns the shared in-memory store, lookup, listing,
  registration, rename, removal, and rollback-aware mutation logic.

The API layer does not own catalog invariants or playback policy. The catalog
does not know about HTTP or audio playback.

## Catalog model

```text
Store = Arc<Mutex<HashMap<u64, SoundMeta>>>
```

The catalog loads `<data_dir>/digest.json` once at startup, where `data_dir`
comes from `resolve_data_dir()`: `PERUN_DATA_DIR` is used unchanged when set;
otherwise Perun tries `/var/lib/perun` and falls back to `./soundbank` if the
default cannot be created. `catalog::init()` creates `<data_dir>/sounds/` for
uploads, `list()` returns entries sorted by id, and `find()` accepts exactly
one id or title selector. Mutations save through a temporary manifest and
restore memory on save failure.

The current metadata fields are:

```text
id, title, duration, path, uploaded_date
```

Duration remains the chosen display format (`"m:ss"`).

## Request flow

```text
HTTP request
→ route-level upload body limit where applicable
→ sound handler validates/extracts input
→ catalog resolves or mutates metadata
→ playback requests enqueue PlaybackCommand on the bounded queue
→ handler maps success or ApiError to HTTP
                    ↓
              audio worker drains queue
              → spawn_blocking
              → keep OutputStream alive
              → rodio sink playback
```

The `/play` request returns immediately instead of waiting for audio completion.

## Configuration

Settings are read from environment variables at startup. `PERUN_DATA_DIR` is
kept unchanged when set; otherwise the service tries `/var/lib/perun` and
falls back to `./soundbank` if needed. The selected path is printed at startup:

```text
PERUN_BIND=0.0.0.0:3030
PERUN_MAX_UPLOAD_MB=10
PERUN_MAX_TITLE_CHARS=100
PERUN_DATA_DIR=/var/lib/perun
```

Invalid numeric settings fail startup instead of silently falling back.
Uploaded files live under `<data_dir>/sounds/` and metadata is stored in
`<data_dir>/digest.json`.

Playback is implemented through a bounded queue plus `rodio`:

```text
request → catalog lookup → PlaybackCommand { title, path } → mpsc queue (size 16) → 202 Accepted
                                              ↓
                                     audio worker → spawn_blocking → keep OutputStream alive → fresh rodio sink → sleep_until_end()
```

The handler returns `500` if the queue is full or the worker is closed.

## Current safety behavior

- malformed multipart input returns `400`, not a panic;
- uploads are protected by a configurable request body limit;
- uploaded file bytes and title length are checked in the handler;
- failed registration removes its newly written file;
- catalog removal is persisted before file deletion;
- missing files during deletion are treated as already absent;
- playback requests still return immediately, but the worker blocks on `sleep_until_end()` so each sound reaches the device before the next command is processed;
- the worker keeps the `OutputStream` alive so rodio can reach the device;
- full or closed playback queues map to `500`;
- worker failures are logged with real error text and the queue keeps draining;
- poisoned catalog mutexes are recovered;
- unwritable default data-dir environments fall back to `./soundbank` instead of panicking;
- API errors are normalized through `ApiError` into JSON `{"error":"..."}` bodies.

Remaining hardening includes Docker packaging.
