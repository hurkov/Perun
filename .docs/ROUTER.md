# Perun Documentation Router

Perun is a Rust HTTP service for low-latency sound notifications. It is
planned for Docker on a Linux LAN.

## Status

The catalog/API foundation is still in place, and playback is now fully fixed.
`src/audio/worker.rs` uses `sink.sleep_until_end()` inside `spawn_blocking`, so
the worker keeps the sink alive until the sound finishes instead of dropping it
after a non-blocking `empty()` check. The `played sound` log now lands after
real playback completes, while `POST /play` still returns `202` immediately
after enqueueing.
Temporary example files (`examples/timetest.rs` and the rest of `examples/`)
were removed. Verification passed: `cargo fmt --check`, `cargo test` (16
tests), `cargo check`, and a live smoke test on `127.0.0.1:3061` where the
`played sound` log arrived ~11s after the request. Next work: Docker
packaging.

## Navigation

| Area | Document | Description |
|---|---|---|
| Progress | `progress/current-focus.md` | Current implementation state |
| Progress | `progress/next-steps.md` | Ordered upcoming work |
| Progress | `progress/completed.md` | Finished milestones |
| Decisions | `decisions/decision-log.md` | Design choices and rationale |
| Technical | `technical/architecture.md` | Source tree and responsibilities |
| Technical | `technical/api.md` | HTTP routes and request behavior |
| Technical | `technical/audio.md` | Playback queue, worker, and rodio policy |
| Technical | `technical/deployment.md` | Docker, Linux, LAN, and storage |
| Technical | `technical/development.md` | Commands, dependencies, roadmap |
| Technical | `technical/logging.md` | Logging status and policy |
| Tree Mirror | `tree/ROUTE.md` | Literal project-tree documentation mirror |

## Documentation tree

```text
.docs/
├── ROUTER.md / ROUTER.json       # navigation and machine-readable tree
├── progress/                     # implementation status
├── decisions/                    # architecture decisions
├── technical/                    # technical reference
└── tree/                         # literal project-tree documentation mirror
```

## Source tree

```text
src/
├── main.rs                       # startup and server lifecycle
├── app.rs                        # top-level /api mounting, queue creation, worker spawn
├── audio/                        # playback command type and worker
│   ├── mod.rs                    # playback command re-export
│   └── worker.rs                 # bounded playback queue consumer
├── config/mod.rs                 # environment-backed runtime settings with data-dir fallback
├── catalog/                      # sound catalog facade, model, persistence, and store
│   ├── mod.rs                    # public catalog facade
│   ├── errors.rs                 # catalog error types
│   ├── model.rs                  # SoundMeta data model
│   ├── persistence.rs            # configured data directory and digest JSON persistence
│   └── store.rs                  # in-memory lookup and mutation logic, plus unit tests
└── api/
    ├── error.rs                  # shared API error mapping
    ├── mod.rs                    # API composition, health route, and health test
    └── sounds/
        ├── mod.rs                # sound routes
        ├── types.rs              # request types
        └── handlers.rs           # sound handlers and feature-local tests
```
