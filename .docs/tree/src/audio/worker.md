# `src/audio/worker.rs`

Drains the bounded playback queue, decodes each registered sound in a blocking task, and plays it through a fresh rodio sink while logging failures and continuing.
