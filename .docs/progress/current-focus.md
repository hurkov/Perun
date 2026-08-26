# Current Focus

**Status**: Final playback fix is complete. `src/audio/worker.rs::attempt_play()` now uses `sink.sleep_until_end()` inside its `spawn_blocking` task, so playback stays alive until the sound fully finishes instead of dropping the sink after a non-blocking `empty()` check. Verification passed: `cargo fmt --check`, `cargo test` (16 tests), `cargo check`, and a live smoke test on `127.0.0.1:3061` where the `played sound` log arrived ~11s after the request for the 10s file.

## Now

- Start Docker packaging for Linux audio deployment.
