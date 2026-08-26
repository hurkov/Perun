# Logging

Perun no longer owns file logging.

There is no `src/logging.rs`, no runtime log directory, no `perun.log`, and no
application-managed retention policy. `src/main.rs` now installs
`tracing_subscriber` with `EnvFilter` (`RUST_LOG`, default `info`) so logs go
to stdout/stderr and the audio worker's `info!` / `warn!` messages are visible.
Docker, systemd, or the host process supervisor should capture stdout/stderr
and handle persistence when deployment is added.
