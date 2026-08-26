# `src/config/mod.rs`

Defines runtime settings loaded from environment variables with defaults for the
bind address, upload size limit, maximum title length, and data-directory
resolution. `PERUN_DATA_DIR` is used unchanged when set; otherwise the module
tries `/var/lib/perun` and falls back to `./soundbank`, and startup prints the
chosen data directory.
