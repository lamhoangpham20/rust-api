# Build and Run a Rust REST API from Scratch

Nov 10, 2026 · Lam Pham

**CONFIDENTIAL — Qt Group Internal Use Only** · Version v0.1

> **AI generation disclaimer.** This document was produced with Claude AI assistance within Qt Group's internal Claude environment. Commands, versions and code must be checked by a qualified human reviewer before use in any business decision, customer communication or external deliverable. Do not distribute outside Qt Group without prior written approval.

## What you'll build

You will build a Todo REST API in Rust, starting from an empty folder. It uses [Axum](https://github.com/tokio-rs/axum) for routing and [Tokio](https://tokio.rs) as the async runtime. It runs on `http://localhost:3000` and keeps its data in memory. Plan on about 30 minutes.

| Method | Path | What it does | Success code |
| --- | --- | --- | --- |
| GET | `/health` | Liveness check | 200 |
| GET | `/todos` | List all todos | 200 |
| POST | `/todos` | Create a todo | 201 |
| GET | `/todos/{id}` | Fetch one todo | 200 (404 if missing) |
| PATCH | `/todos/{id}` | Update title and/or completed | 200 (404 if missing) |
| DELETE | `/todos/{id}` | Delete a todo | 204 (404 if missing) |

Final project layout:

```text
rust-api/
├── Cargo.toml        # package manifest + dependencies
├── Cargo.lock        # exact resolved versions (commit this)
├── src/
│   ├── main.rs       # starts the server
│   ├── lib.rs        # router + shared state
│   ├── models.rs     # data types
│   └── handlers.rs   # one function per endpoint
└── tests/
    └── api.rs        # integration tests
```

## Prerequisites

You need the Rust toolchain (`rustc` + `cargo`), installed with [rustup](https://rustup.rs), plus `curl` for testing.

1. Install Rust on macOS or Linux:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

   On Windows, download and run `rustup-init.exe` from https://rustup.rs. It will also ask you to install the Visual Studio C++ Build Tools.
2. macOS only: if the build later fails with `linker 'cc' not found`, install the Xcode command line tools with `xcode-select --install`.
3. Open a new terminal, or run `source "$HOME/.cargo/env"`, so `cargo` is on your `PATH`.
4. Check the install:

   ```bash
   rustc --version
   cargo --version
   ```
5. Optional: install the **rust-analyzer** extension for VS Code (or use RustRover). It gives you autocomplete and inline type errors.

Run `rustup update` from time to time to stay on the latest stable release.

## Step 1 — Create the project

Run `cargo new` to create a working "Hello, world!" binary crate with its own git repo.

```bash
cargo new rust-api
cd rust-api
```

If you already have an empty `rust-api` folder, run `cargo init` inside it instead.

Cargo generates two files. `Cargo.toml` is the manifest: package name, Rust edition and dependencies.

```toml
[package]
name = "rust-api"
version = "0.1.0"
edition = "2024"

[dependencies]
```

`src/main.rs` is the entry point:

```rust
fn main() {
    println!("Hello, world!");
}
```

Run it. The first build compiles the project into `target/debug/`.

```bash
cargo run
```

```text
   Compiling rust-api v0.1.0 (.../rust-api)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.35s
     Running `target/debug/rust-api`
Hello, world!
```

The package name `rust-api` becomes the crate name `rust_api` in code (Rust swaps the dash for an underscore). You will use that name in Step 5.

## Step 2 — Add dependencies

Use `cargo add` to add the dependencies. It fetches the latest compatible version and writes it to `Cargo.toml`.

```bash
cargo add axum
cargo add tokio --features full
cargo add serde --features derive
cargo add uuid --features v4,serde
cargo add tracing tracing-subscriber --features tracing-subscriber/env-filter

# used only by the tests in Step 7
cargo add --dev tower --features util
cargo add --dev http-body-util serde_json
```

| Crate | Why you need it |
| --- | --- |
| `axum` | Web framework: routing, extractors, JSON responses |
| `tokio` | Async runtime that runs the server |
| `serde` | Converts Rust structs to and from JSON |
| `uuid` | Random IDs for todos (`v4`), serializable (`serde`) |
| `tracing`, `tracing-subscriber` | Structured logging, controlled by `RUST_LOG` |
| `tower`, `http-body-util`, `serde_json` | Dev-only: call the router in tests without opening a port |

Your `Cargo.toml` should now look like this. These are the versions tested for this guide; newer patch versions are fine.

```toml
[dependencies]
axum = "0.8.9"
serde = { version = "1.0.229", features = ["derive"] }
tokio = { version = "1.54.1", features = ["full"] }
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.23", features = ["env-filter"] }
uuid = { version = "1.29.2", features = ["v4", "serde"] }

[dev-dependencies]
http-body-util = "0.1.5"
serde_json = "1.0.154"
tower = { version = "0.5.3", features = ["util"] }
```

## Step 3 — A first server with a health endpoint

Start with a server that has one route, so you can check the toolchain and dependencies before adding logic. Replace `src/main.rs` with:

```rust
use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(|| async { "OK" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}
```

- `#[tokio::main]` starts the async runtime and lets `main` be `async`.
- `Router::new().route(path, get(handler))` maps `GET /health` to a handler. Any async function or closure whose return value converts into a response can be a handler.
- `axum::serve` accepts connections until you press Ctrl+C.

Run it. The first build downloads and compiles the dependencies, which takes about a minute.

```bash
cargo run
```

In a second terminal:

```bash
curl -i http://localhost:3000/health
```

```text
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 2

OK
```

Stop the server with Ctrl+C before you go on.

## Step 4 — Data model and shared state

A todo is a plain struct with serde derives. Request bodies get their own structs, so clients can't set fields like `id`. Create `src/models.rs`:

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A todo item as stored and returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
}

/// Request body for POST /todos.
#[derive(Debug, Deserialize)]
pub struct CreateTodo {
    pub title: String,
}

/// Request body for PATCH /todos/{id}. Every field is optional.
#[derive(Debug, Deserialize)]
pub struct UpdateTodo {
    pub title: Option<String>,
    pub completed: Option<bool>,
}
```

For storage, every request handler needs access to the same data. The usual pattern is `Arc<RwLock<HashMap<..>>>`:

- `HashMap<Uuid, Todo>` holds the todos, keyed by ID.
- `RwLock` allows many readers or one writer at a time, so concurrent requests are safe.
- `Arc` is a shared pointer. Each handler gets a cheap clone that points to the same map.

You'll define this as a type alias, `Db`, in `src/lib.rs` in the next step. The data is lost when the server stops; swapping in a real database is covered under Next steps.

## Step 5 — Handlers, router and main

The code goes in three files: handlers (one function per endpoint), the router in a library crate, and a short `main.rs`. Putting the router in `lib.rs` means the tests in Step 7 can import it.

**`src/handlers.rs`.** Extractors such as `State`, `Path` and `Json` pull typed values out of the request. Return types such as `(StatusCode, Json<T>)` or `Result<Json<T>, StatusCode>` become the response.

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    models::{CreateTodo, Todo, UpdateTodo},
    Db,
};

pub async fn health() -> &'static str {
    "OK"
}

pub async fn list_todos(State(db): State<Db>) -> Json<Vec<Todo>> {
    let todos = db.read().unwrap().values().cloned().collect();
    Json(todos)
}

pub async fn create_todo(
    State(db): State<Db>,
    Json(input): Json<CreateTodo>,
) -> (StatusCode, Json<Todo>) {
    let todo = Todo {
        id: Uuid::new_v4(),
        title: input.title,
        completed: false,
    };
    db.write().unwrap().insert(todo.id, todo.clone());
    (StatusCode::CREATED, Json(todo))
}

pub async fn get_todo(
    State(db): State<Db>,
    Path(id): Path<Uuid>,
) -> Result<Json<Todo>, StatusCode> {
    db.read()
        .unwrap()
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn update_todo(
    State(db): State<Db>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateTodo>,
) -> Result<Json<Todo>, StatusCode> {
    let mut store = db.write().unwrap();
    let todo = store.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;

    if let Some(title) = input.title {
        todo.title = title;
    }
    if let Some(completed) = input.completed {
        todo.completed = completed;
    }
    Ok(Json(todo.clone()))
}

pub async fn delete_todo(State(db): State<Db>, Path(id): Path<Uuid>) -> StatusCode {
    match db.write().unwrap().remove(&id) {
        Some(_) => StatusCode::NO_CONTENT,
        None => StatusCode::NOT_FOUND,
    }
}
```

**`src/lib.rs`.** This file defines the shared state and wires paths to handlers. Axum 0.8 writes path parameters as `{id}`; older versions used `:id`.

```rust
pub mod handlers;
pub mod models;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::{routing::get, Router};
use uuid::Uuid;

use models::Todo;

/// Shared, thread-safe in-memory store. Cloning it is cheap (Arc).
pub type Db = Arc<RwLock<HashMap<Uuid, Todo>>>;

/// Builds the router. Kept separate from main() so tests can call it.
pub fn app() -> Router {
    let db: Db = Arc::default();

    Router::new()
        .route("/health", get(handlers::health))
        .route("/todos", get(handlers::list_todos).post(handlers::create_todo))
        .route(
            "/todos/{id}",
            get(handlers::get_todo)
                .patch(handlers::update_todo)
                .delete(handlers::delete_todo),
        )
        .with_state(db)
}
```

**`src/main.rs`.** Replace the Step 3 version. It now sets up logging, reads the port from `PORT` (default 3000), and serves `rust_api::app()`.

```rust
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Logging: override with e.g. RUST_LOG=debug cargo run
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("listening on http://{addr}");

    axum::serve(listener, rust_api::app()).await.unwrap();
}
```

Check that it compiles: `cargo build` should finish with no errors or warnings.

## Step 6 — Run and call the API

Start the server. Your IDs will differ from the ones below.

```bash
cargo run
```

```text
INFO rust_api: listening on http://0.0.0.0:3000
```

In a second terminal, go through the full lifecycle of one todo:

```bash
# Create a todo -> 201
curl -s -X POST http://localhost:3000/todos \
  -H 'content-type: application/json' \
  -d '{"title":"Learn Axum"}'
# {"id":"d0b6162a-ebe6-4fb9-888e-4b874402cb81","title":"Learn Axum","completed":false}

# Copy the id into a variable
ID=d0b6162a-ebe6-4fb9-888e-4b874402cb81

# List -> 200
curl -s http://localhost:3000/todos
# [{"id":"d0b6...","title":"Learn Axum","completed":false}]

# Mark done -> 200
curl -s -X PATCH http://localhost:3000/todos/$ID \
  -H 'content-type: application/json' \
  -d '{"completed":true}'
# {"id":"d0b6...","title":"Learn Axum","completed":true}

# Delete -> 204, then fetching it -> 404
curl -s -o /dev/null -w '%{http_code}\n' -X DELETE http://localhost:3000/todos/$ID
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:3000/todos/$ID
```

Axum's extractors reject bad input on their own, before your handler runs:

| Bad request | Status | Response body |
| --- | --- | --- |
| Body missing `title`: `{"name":"x"}` | 422 | `Failed to deserialize the JSON body into the target type: missing field 'title'` |
| ID that isn't a UUID: `/todos/abc` | 400 | `Invalid URL: Cannot parse 'id' with value 'abc': UUID parsing failed` |
| No `content-type: application/json` header | 415 | `Expected request with Content-Type: application/json` |

To change the port or the log level: `PORT=8080 RUST_LOG=debug cargo run`. If you prefer a GUI, the same calls work from Postman, Insomnia or the VS Code REST Client extension.

## Step 7 — Automated tests

Integration tests call the router directly with `oneshot` from `tower::ServiceExt`. There's no port and no running server, so the tests are fast and can run in parallel. Each test calls `app()`, which creates a fresh, empty store. Create `tests/api.rs`:

```rust
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt; // for `oneshot`

#[tokio::test]
async fn health_returns_ok() {
    let app = rust_api::app();

    let res = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn create_then_fetch_todo() {
    let app = rust_api::app();

    // Create
    let res = app
        .clone()
        .oneshot(
            Request::post("/todos")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "title": "Write tests" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let created: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(created["title"], "Write tests");
    assert_eq!(created["completed"], false);

    // Fetch it back by id
    let id = created["id"].as_str().unwrap();
    let res = app
        .oneshot(Request::get(format!("/todos/{id}")).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_todo_returns_404() {
    let app = rust_api::app();

    let res = app
        .oneshot(
            Request::get("/todos/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
```

Run the tests:

```bash
cargo test
```

```text
running 3 tests
test unknown_todo_returns_404 ... ok
test health_returns_ok ... ok
test create_then_fetch_todo ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Before you commit, run `cargo fmt` to format the code and `cargo clippy` to lint it. If either is missing, add it with `rustup component add rustfmt clippy`.

## Step 8 — Release build and Docker image

`cargo run` builds an unoptimized debug binary. To deploy, build with optimizations:

```bash
cargo build --release
./target/release/rust-api
```

The result is one self-contained binary of about 3 MB, with no runtime to install. Copy it to any machine with the same OS and CPU architecture.

To ship it as a container, add a multi-stage `Dockerfile`. The first stage compiles the code; the second copies only the binary into a slim Debian image.

```dockerfile
# ---- build stage ----
FROM rust:1-slim-trixie AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# ---- runtime stage ----
FROM debian:trixie-slim
COPY --from=builder /app/target/release/rust-api /usr/local/bin/rust-api
ENV PORT=3000
EXPOSE 3000
CMD ["rust-api"]
```

Add a `.dockerignore` so the large local `target/` folder isn't sent to Docker:

```text
target
.git
```

Build and run:

```bash
docker build -t rust-api .
docker run --rm -p 3000:3000 rust-api
curl http://localhost:3000/health   # OK
```

The final image is about 144 MB, mostly the Debian base. Keep the builder and runtime images on the same Debian release (`trixie` here) so the binary's glibc version matches.

## Troubleshooting and next steps

| Symptom | Cause | Fix |
| --- | --- | --- |
| `command not found: cargo` | `~/.cargo/bin` isn't on `PATH` | Open a new terminal or run `source "$HOME/.cargo/env"` |
| `linker 'cc' not found` (macOS) | No C toolchain | `xcode-select --install` |
| `Address already in use` panic at startup | Port 3000 is taken, often by a previous run | Stop the old process (`lsof -i :3000`) or run with `PORT=3001` |
| `unresolved import rust_api` | `src/lib.rs` is missing, or the package isn't named `rust-api` | Create `lib.rs` as in Step 5, or use your own crate name (dashes become underscores) |
| Panic: path segments must not start with `:` | Axum 0.7-style `/todos/:id` route | Use `/todos/{id}` in Axum 0.8 |
| `the trait Handler<_, _> is not implemented` | A handler isn't `async`, or `Json(..)` isn't the last argument | Make it `async fn` and put the body extractor last |
| curl returns 415 | Missing JSON header | Add `-H 'content-type: application/json'` |

Once the basics work, these are the usual next steps:

- **Persistence:** replace the `HashMap` with PostgreSQL or SQLite through [sqlx](https://github.com/launchbadge/sqlx), using a connection pool as the state.
- **Error handling:** define an `AppError` enum that implements `IntoResponse` and returns JSON error bodies. Also replace `.unwrap()` on the lock.
- **Validation:** reject empty titles, e.g. with the [validator](https://crates.io/crates/validator) crate.
- **Middleware:** use [tower-http](https://crates.io/crates/tower-http) for request logging (`TraceLayer`), CORS and timeouts.
- **API docs:** generate an OpenAPI spec and Swagger UI with [utoipa](https://github.com/juhaku/utoipa).
- **Graceful shutdown:** call `axum::serve(..).with_graceful_shutdown(signal)` so in-flight requests finish on Ctrl+C or SIGTERM.

## References

Every step was tested on 10 November 2026 on macOS with Rust 1.99.1 and Cargo 1.99.1: build, all 3 tests, each curl call and the Docker image. Other content is based on Claude's general knowledge; check versions before external use.

1. Tool: rustup installer — https://rustup.rs
2. Crate: axum (web framework) — https://github.com/tokio-rs/axum
3. Crate: tokio (async runtime) — https://tokio.rs
4. Crate: serde — https://serde.rs
5. Crate: sqlx (database, next steps) — https://github.com/launchbadge/sqlx
6. Crate: utoipa (OpenAPI, next steps) — https://github.com/juhaku/utoipa
7. Reference: Cargo manifest format — https://doc.rust-lang.org/cargo/reference/manifest.html

## Version history

| Version | Date | Author(s) | Changes |
| --- | --- | --- | --- |
| v0.1 | 10 November 2026 | Lam Pham | Initial draft — generated with Claude AI |

*Generated with Claude AI (qt-document-annotation) — validate all commands and versions before use.*
