# Deployment

## Target environment

Perun is designed to run in a Docker container on Linux, accessible via LAN.

## Deployment decisions

- **Platform**: Linux (Docker)
- **Network**: LAN-only, no WAN exposure
- **Authentication**: None (LAN is the trusted boundary)
- **Configuration**: Environment variables
- **Storage**: Docker volume for persistent data under `/var/lib/perun`
- **Audio**: Linux audio device access (ALSA/PulseAudio/PipeWire); playback uses `rodio 0.20` with MP3 support inside `src/audio/worker.rs`

## Configuration

Environment variables are optional. If they are not provided, defaults are used:

```bash
PERUN_BIND=0.0.0.0:3030           # Bind address and port; default shown
PERUN_MAX_UPLOAD_MB=10            # Upload request/file limit in MiB; default shown
PERUN_MAX_TITLE_CHARS=100         # Maximum uploaded sound title length; default shown
PERUN_DATA_DIR=/var/lib/perun     # Persistent data directory
```

Invalid numeric values fail startup. `PERUN_DATA_DIR` is used unchanged when
set; otherwise Perun tries to create `/var/lib/perun` and falls back to
`./soundbank` if that default is not writable. Startup prints the selected data
dir as `perun: data dir: {path}`. Uploaded files are stored under
`<data_dir>/sounds/` and metadata lives at `<data_dir>/digest.json`.

## Docker usage (planned)

Basic run:
```bash
docker run -d \
  --name perun \
  -p 3030:3030 \
  -v perun-data:/var/lib/perun \
  perun/perun:latest
```

With audio device access:
```bash
docker run -d \
  --name perun \
  -p 3030:3030 \
  -v perun-data:/var/lib/perun \
  --device /dev/snd \
  perun/perun:latest
```

Custom configuration:
```bash
docker run -d \
  --name perun \
  -p 8080:9090 \
  -e PERUN_BIND=0.0.0.0:9090 \
  -e PERUN_MAX_UPLOAD_MB=20 \
  -e PERUN_MAX_TITLE_CHARS=120 \
  -e PERUN_DATA_DIR=/var/lib/perun \
  -v perun-data:/var/lib/perun \
  perun/perun:latest
```

## Volume structure

```
/var/lib/perun/
├── sounds/      # Uploaded audio files
└── digest.json  # Catalog metadata
```

## Security notes

**⚠️ LAN-only assumption**

Perun currently has no authentication. It assumes:
- Running on a trusted LAN
- Firewall/router blocks WAN access
- Docker port not exposed to internet
- Network isolation provides security boundary

**Do not expose directly to the internet.**

If deployment assumptions change, authentication can be added via middleware without rewriting the core application.

## Public Docker image

Future goal: publish official `perun/perun` image for easy deployment.

Requirements:
- Minimal image size
- Clear documentation
- Volume persistence examples
- Linux audio setup guide
- Healthcheck endpoint
- Non-root user (if audio permissions allow)
