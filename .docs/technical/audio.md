# Audio Playback

## Purpose

Own the runtime playback queue and worker. `src/audio/mod.rs` defines and re-exports `PlaybackCommand`; `src/audio/worker.rs` drains the queue and performs playback.

## Current source tree

```text
src/audio/
├── mod.rs        # playback command type and module re-export
└── worker.rs     # bounded queue consumer and rodio playback loop
```

## Responsibilities

- `PlaybackCommand` carries the resolved sound `title` and file `path`.
- `app.rs` creates the bounded `tokio::sync::mpsc` queue and spawns the worker.
- `api/sounds/handlers.rs` turns `/play` requests into commands and enqueues them.
- `worker.rs` plays each command on its own rodio sink inside `spawn_blocking`, keeping the `OutputStream` alive for the whole playback and waiting with `sleep_until_end()`.
- File, decode, and audio-device errors are logged with their real rodio error text, and the worker keeps draining.
- The backend is `rodio 0.20` with MP3 support.

## Playback policy

```text
HTTP /play
→ catalog lookup
→ PlaybackCommand { title, path }
→ bounded queue (size 16)
→ 202 Accepted

worker
→ recv command
→ spawn_blocking
→ open file / decode / create sink / play / sleep_until_end()
```

Each command gets its own sink, the stream owner stays alive until playback completes, and `sleep_until_end()` keeps the blocking task parked until the last sample is heard, so one bad file or device error does not stop the queue.
