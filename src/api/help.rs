use axum::http::header::CONTENT_TYPE;
use axum::response::Response;
const HELP: &str = r#"Perun API
=======

GET /health
Check that the service is up.
Example: curl http://127.0.0.1:3030/health

GET /sounds/library
List all sounds as JSON.
Example: curl http://127.0.0.1:3030/sounds/library

POST /sounds/upload
Upload a sound. Multipart form fields: title (text), file (audio file).
Example: curl -X POST http://127.0.0.1:3030/sounds/upload -F 'title=door' -F 'file=@door.mp3'

GET /sounds/play
Play a sound. Use exactly one selector: ?id=123 or ?title=name.
Returns 202 queued; the sound plays asynchronously, and the server log shows completion.
Examples:
  curl 'http://127.0.0.1:3030/sounds/play?id=123'
  curl 'http://127.0.0.1:3030/sounds/play?title=door'

PATCH /sounds/rename
Rename a sound. Exactly one selector in the query (?id= or ?title=),
JSON body: {"title": "new name"}.
Example: curl -X PATCH 'http://127.0.0.1:3030/sounds/rename?id=123' -H 'Content-Type: application/json' -d '{"title": "new name"}'

DELETE /sounds/delete
Delete a sound (file + metadata). Exactly one selector: ?id=123 or ?title=name.
Example: curl -X DELETE 'http://127.0.0.1:3030/sounds/delete?id=123'

Errors
======
All errors are JSON: {"error": "..."}
Status codes: 400 (bad request), 404 (not found), 409 (conflict), 413 (too large), 500 (internal).
"#;

pub async fn handle() -> Response {
    Response::builder()
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(axum::body::Body::from(HELP.to_string()))
        .expect("static response is valid")
}
