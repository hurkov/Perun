# API

## Base URL

The current development default is:

```text
http://localhost:3030/api
```

Selector-based operations require exactly one of `id` or `title`.

All API errors return JSON `{"error":"..."}` bodies via the shared `ApiError` type.

## Bruno collection

Ready-to-use Bruno requests live under `../../bruno/`. Select the `local`
environment and adjust `uploadFilePath` before using the multipart upload
request.

## Routes

### `GET /api/health`

Returns `200 OK` with an empty body.

### `POST /api/sounds/upload`

Multipart fields:

- `title`: unique, non-empty sound title;
- `file`: audio bytes, currently validated through `lofty`.

The handler trims the title, enforces the configured title limit, sanitizes the
filename, parses duration in memory, checks the configured upload size, writes
the file, and registers metadata. Failed registration removes the newly written
file.

Upload limits are controlled by environment variables at startup:

```text
PERUN_MAX_UPLOAD_MB=10
PERUN_MAX_TITLE_CHARS=100
```

Responses: `200`, `400`, `409`, `413`, or `500`.

```bash
curl -i \
  -F "title=Doorbell" \
  -F "file=@doorbell.mp3" \
  http://localhost:3030/api/sounds/upload
```

### `GET /api/sounds/library`

Returns the in-memory catalog sorted by id.

### `GET /api/sounds/play`

Looks up the requested sound, enqueues a `PlaybackCommand { title, path }`, and
returns `202 Accepted` immediately with `playback queued`.

If the queue is full or the worker is closed, the handler returns `500`.
The existing selector validation stays the same:

- `400` for an invalid selector;
- `404` when the sound does not exist.

```bash
curl -i "http://localhost:3030/api/sounds/play?id=1"
curl -i "http://localhost:3030/api/sounds/play?title=Doorbell"
```

### `DELETE /api/sounds/delete`

Deletes by exactly one id or title. Metadata is persisted before the file is
removed.

```bash
curl -i -X DELETE "http://localhost:3030/api/sounds/delete?id=1"
```

### `PATCH /api/sounds/rename`

The query selects the existing sound and the JSON body contains the new title.

```bash
curl -i -X PATCH \
  "http://localhost:3030/api/sounds/rename?id=1" \
  -H "Content-Type: application/json" \
  -d '{"title":"New Doorbell"}'
```

Responses: `200` success, `400` invalid selector/title, `404` missing sound,
`409` duplicate title, or `500` persistence failure.

## Future routes

- WebSocket transport using the same playback command channel.
