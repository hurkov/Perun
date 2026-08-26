# Development

## Commands

```bash
cargo build            # build
cargo run              # run the server (listens on 0.0.0.0:3030)
                       # falls back to ./soundbank when /var/lib/perun is unwritable
cargo check            # fast type-check
cargo clippy           # lint
cargo fmt              # format
cargo test             # run all tests (catalog store, API, and playback handler tests)
cargo test <name>      # run a single test by name substring
```

Rust edition 2024. Dependencies: axum (multipart), tokio, tower-http (trace), tracing, tracing-subscriber, serde (derive), serde_json, lofty (audio file duration/metadata parsing), rodio (0.20, MP3 feature).

Only one instance can run at a time — the port bind panics with `AddrInUse` if another instance is already listening. Check with `lsof -nP -iTCP:3030 -sTCP:LISTEN`.

On plain local runs, the selected data directory is printed at startup so it's
obvious whether Perun is using `/var/lib/perun` or the `./soundbank` fallback.

## Roadmap

### Phase 1: Catalog/API hardening ✅

**Status: Complete**

1. **Compile successfully** — catalog and sound handlers build cleanly ✅
2. **Fix upload consistency** — prevent orphaned files on registration failure ✅
3. **Fix delete consistency** — prevent stale catalog entries on save failure ✅
4. **Remove panic paths** — replace request-input `.unwrap()` calls with proper error responses ✅
5. **Split sound feature files** — keep routes, request types, and handlers readable ✅

### Phase 2: Configuration and hardening ✅

**Status: Complete**

6. **Config module** — `PERUN_BIND`, `PERUN_MAX_UPLOAD_MB`, `PERUN_MAX_TITLE_CHARS`, `PERUN_DATA_DIR` fallback ✅
7. **Upload limits** — enforce max size, reject empty/huge titles ✅
8. **Add tests** — catalog store unit tests and feature-based API handler tests are now present and verified: 7 `catalog::store` tests, 1 `api::health` test, and 8 `api::sounds::handlers` tests all passed ✅
9. **Async filesystem** — blocking I/O is now handled with `spawn_blocking` ✅
10. **Better error types** — `src/api/error.rs` now centralizes HTTP mapping via `ApiError` ✅

### Phase 3: Playback ✅

**Status: Complete**

11. **Playback worker** — `src/audio/mod.rs` + `src/audio/worker.rs` own the playback queue and consumer ✅
12. **Audio engine** — `rodio`-based playback consumer with MP3 support; keeps the stream alive and waits for `sleep_until_end()` so playback reaches the device ✅
13. **Update `/play` endpoint** — build command, enqueue, return `202 Accepted` immediately; verified by a live smoke test on `127.0.0.1:3061` where the `played sound` log landed ~11s after the request ✅

### Phase 4: Optimization

14. **Title index** — add `HashMap<String, u64>` for instant title lookups
15. **Monotonic IDs** — persist next-id counter to prevent ID reuse after deletion
16. **Sound preloading** — optional in-memory buffer cache for lowest latency

### Phase 5: Docker and deployment

17. **Docker packaging** — minimal image, non-root user, volume support
18. **Docker Compose** — example with volume and audio device
19. **CI/build** — automated image builds
20. **Public image** — publish to Docker Hub

### Future (deferred)

- WebSocket endpoint for streaming/subscriptions
- Client applications (CLI/GUI)
- Multiple audio format support
- Playlist/sequence playback
