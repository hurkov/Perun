# Perun API

Low-latency sound notification service for a LAN. HTTP API + real audio playback.

The live API reference is also served by the running instance at `GET /help`
(e.g. `curl http://127.0.0.1:3030/help`), so it is always in sync with the build.

## Endpoints

Base URL: `http://<host>:3030` (default bind `0.0.0.0:3030`, env `PERUN_BIND`).

### `GET /health`

Liveness check. Returns `200` with no body.

```bash
curl http://127.0.0.1:3030/health
```

### `GET /sounds/library`

Lists all sounds as a JSON array.

```bash
curl http://127.0.0.1:3030/sounds/library
```

### `POST /sounds/upload`

Uploads a sound. Multipart form fields: `title` (text), `file` (audio file).

```bash
curl -X POST http://127.0.0.1:3030/sounds/upload -F 'title=door' -F 'file=@door.mp3'
```

### `GET /sounds/play`

Plays a sound. Expects **exactly one** selector: `?id=123` *or* `?title=name`.
Returns `202` (playback queued); the sound plays asynchronously and the server
log shows completion.

```bash
curl 'http://127.0.0.1:3030/sounds/play?id=123'
curl 'http://127.0.0.1:3030/sounds/play?title=door'
```

### `PATCH /sounds/rename`

Renames a sound. Exactly one selector in the query (`?id=` or `?title=`);
JSON body with the new title.

```bash
curl -X PATCH 'http://127.0.0.1:3030/sounds/rename?id=123' \
  -H 'Content-Type: application/json' \
  -d '{"title": "new name"}'
```

### `DELETE /sounds/delete`

Deletes a sound (metadata + audio file). Exactly one selector.

```bash
curl -X DELETE 'http://127.0.0.1:3030/sounds/delete?id=123'
```

## Errors

All errors are JSON: `{"error": "..."}`.

| Code | Meaning |
|------|---------|
| 400  | bad request (bad selector, invalid title, invalid audio file) |
| 404  | notification sound doesn't exist |
| 409  | a sound with that title already exists |
| 413  | file is too large |
| 500  | internal error |

## Configuration

| Env | Default | Purpose |
|-----|---------|---------|
| `PERUN_BIND` | `0.0.0.0:3030` | listen address |
| `PERUN_DATA_DIR` | `/var/lib/perun` (fallback `./soundbank`) | sound store |
| `PERUN_MAX_UPLOAD_MB` | `10` | upload size cap |
| `PERUN_MAX_TITLE_CHARS` | `100` | title length cap |
| `RUST_LOG` | `info` | log level |

## Deployment

### Option A — with internet (registry)

Build and push once, then pull and run anywhere with internet:

```bash
docker build -t <you>/perun:latest .
docker push <you>/perun:latest
# on the target machine:
docker pull <you>/perun:latest
docker compose up -d
```

### Option B — LAN-only (no internet on the target)

Build, save, copy the image file over, then load:

```bash
# build machine
docker build -t perun:latest .
docker save perun:latest -o perun.img

# copy perun.img + the project (docker-compose.yml) to the target, then:
docker load -i perun.img
docker compose up -d
```

### Audio requirements (Linux)

The container needs access to the host audio device:

```yaml
devices:
  - /dev/snd:/dev/snd
group_add:
  - audio
```

Without `/dev/snd`, requests still succeed (202) but play silently — verify
that the server log prints a `played sound` completion line.
