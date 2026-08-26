# Perun
A small, fast HTTP service that plays sound files over your local network.
Upload an MP3 once, then trigger it by `id` or `title` from anything that can
send an HTTP request — smart-home automations, sensors, scripts, other machines.

- Written in **Rust** (axum + rodio), builds to one static-ish binary.
- Ships as a **Docker image**; audio needs access to the host sound device.
- No database: sounds live on disk, metadata in a single `digest.json`.

Full API reference is served live at `GET /help`, and documented in [API.md](API.md).

---

## Quick start (Docker)

The recommended way to run Perun — the image bundles the ALSA runtime and
handles the persistent data directory for you.

```bash
# 1. Pull the image from Docker Hub
docker pull hurkov/perun:latest

# 2. Run it with the audio device + a persistent volume
docker run -d \
  --name perun \
  -p 3030:3030 \
  -v perun-data:/var/lib/perun \
  --device /dev/snd \
  hurkov/perun:latest
```

The `--device /dev/snd` flag is what gives the container access to your sound
card. Without it the API still works, but playback will be silent.

Or use the bundled compose file (it already wires the device, volume, and
`audio` group):

```bash
docker compose up -d
```

> Note: the bundled `docker-compose.yml` uses `build: .` (build, then run).
> To run the published image with compose, set
> `image: hurkov/perun:latest` and drop the `build:` line.

### Build your own image (optional)

To run a development build or the current source instead of the published
image:

```bash
docker build -t perun .
```

### LAN-only / no-internet host

If the target box can't reach a registry, build + save on a machine that can,
copy the image file over, then load:

```bash
# build machine
docker build -t hurkov/perun:latest .
docker save hurkov/perun:latest -o perun.img

# target box
docker load -i perun.img
docker compose up -d
```

Confirm it's up (the URL prints in the startup logs too):

```bash
curl http://<host>:3030/health      # 200, empty
```

---

## Run from source (local dev)

You need the Rust toolchain and the ALSA development headers (Linux). On
Debian/Ubuntu:

```bash
sudo apt-get install -y build-essential pkg-config libasound2-dev
cargo run
```

On macOS the audio stack differs; the HTTP API works and you can develop
against it, but real playback is a Linux/ALSA feature.

Useful commands:

```bash
cargo build --release    # produce target/release/perun
cargo test               # run the test suite
```

---

## Configuration

All settings are environment variables with sane defaults — you can run with
none set at all.

| Variable          | Default                    | Meaning                          |
|-------------------|----------------------------|----------------------------------|
| `PERUN_BIND`      | `0.0.0.0:3030`             | Listen address and port          |
| `PERUN_DATA_DIR`  | `/var/lib/perun` → `./soundbank` | Where sounds + metadata are stored |
| `PERUN_MAX_UPLOAD_MB` | `10`                   | Upload size limit (MiB)          |
| `PERUN_MAX_TITLE_CHARS` | `100`                 | Max sound title length           |
| `RUST_LOG`        | `info`                     | Log level                        |

`PERUN_DATA_DIR` is used as-is when set. If it's unset, Perun tries to create
`/var/lib/perun`; if that path isn't writable (typical on a plain laptop) it
falls back to `./soundbank` and prints the choice. Uploaded audio files go to
`<data_dir>/sounds/`, metadata to `<data_dir>/digest.json`.

---

## Basic usage

```bash
# upload a sound (multipart: title + file)
curl -X POST http://<host>:3030/sounds/upload \
  -F 'title=door' -F 'file=@door.mp3'

# list the library
curl http://<host>:3030/sounds/library

# play by id or title (exactly one) — returns 202, plays in the background
curl 'http://<host>:3030/sounds/play?title=door'
```

Errors are JSON: `{"error": "..."}` with status `400/404/409/413/500`.

---

## Project layout

```
src/
  main.rs       entrypoint; config, catalog init, banner, axum serve
  app.rs        wires router + audio worker, builds the app
  config/       env-var parsing (PERUN_*) with defaults + data-dir fallback
  catalog/      sound metadata store: digest.json + per-sound files
  audio/        rodio playback worker (background, non-blocking)
  api/          HTTP routes: /health, /help, /sounds/*
```

See [API.md](API.md) for the complete endpoint reference.
