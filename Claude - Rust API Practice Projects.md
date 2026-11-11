# Claude - Rust API Practice Projects

**CONFIDENTIAL — Qt Group Internal Use Only**

Date: 10 November 2026 · Version: v0.1 · Author(s): Lam Pham

> **AI GENERATION DISCLAIMER**
> This document was produced with Claude AI assistance within Qt Group's internal Claude environment. Crate names, versions, and recommendations must be checked by a qualified human reviewer before use in any business decision, customer communication or external deliverable. This document is CONFIDENTIAL and intended for Qt Group internal use only. Do not distribute outside Qt Group without prior written approval.

---

## Executive Summary

**Purpose.** This is a learning path of eight small Rust REST API projects. Each one adds a few new skills, so by the end you have used the full set needed for production backend work.

**Framework recommendation.** Use **Axum**. It is maintained by the Tokio team and built on the `tower` middleware ecosystem, and it has the most momentum in the Rust web space. Your current `rust-api` Todo project already uses it. Actix Web is a strong second choice. Rocket and Loco are worth knowing about but are less suited to learning the fundamentals.

**The path.**

| # | Project | Main new skills | Time |
|---|---|---|---|
| 1 | URL Shortener | Routing, shared state, redirects, validation | 1–2 days |
| 2 | Todo API v2 (Postgres) | `sqlx`, migrations, pagination, error type | 2–3 days |
| 3 | Auth Service | Password hashing, JWT, custom extractors | 3–4 days |
| 4 | Rate-Limiting Reverse Proxy | `tower` layers, writing middleware, `reqwest` | 2–3 days |
| 5 | File Upload & Thumbnail Service | Multipart, streaming, `spawn_blocking` | 2–3 days |
| 6 | Real-Time Chat | WebSockets, `broadcast` channels | 3 days |
| 7 | Background Job Queue | Workers, `SKIP LOCKED`, graceful shutdown | 3–4 days |
| 8 | Capstone: Expense Tracker | OpenAPI, observability, Docker, CI | 1–2 weeks |

**Main advice.** Use one layered architecture for every project (see the section *Shared Architecture*). Write integration tests from project 1 onward. Before starting each new project, rebuild the previous one from memory without looking at your old code.

> ⚠️ Data in this section requires human validation.

---

## 1. Which Framework Is Best?

| Framework | Strengths | Weaknesses | Verdict |
|---|---|---|---|
| **Axum** | Tokio team; `tower` middleware works with it out of the box; extractors are plain types, with no macros; largest current ecosystem | Compile errors from handler trait bounds can be confusing (add `#[axum::debug_handler]` while learning) | ✅ **Best default for learning and production** |
| **Actix Web** | Very fast, mature, many features, good docs | Uses its own actor/runtime layer and its own middleware model, so less of what you learn transfers | ✅ Good alternative |
| **Rocket** | Very ergonomic, macro-based routing, built-in form/config handling | Slower release history; the macros hide how things work | ⚠️ Pleasant, but hides the fundamentals |
| **Loco** | Rails-like full stack built on Axum (generators, ORM, jobs) | Opinionated; hides the parts you want to learn | ⚠️ Try it after this path |
| **Poem / Salvo** | Built-in OpenAPI (Poem), many features | Smaller communities | Niche |

**Why Axum:** everything you learn transfers. Middleware is `tower::Layer`, the HTTP types come from the `http` crate, and the runtime is plain Tokio. The same pieces show up in `tonic` (gRPC), `reqwest`, and most of the async Rust ecosystem.

### Core crate toolbox

| Need | Crate |
|---|---|
| Web framework | `axum`, `axum-extra` |
| Runtime | `tokio` |
| Middleware | `tower`, `tower-http` (CORS, tracing, compression, timeouts, limits) |
| Serialization | `serde`, `serde_json` |
| Database | `sqlx` (compile-time checked SQL), or `sea-orm` if you want an ORM |
| Errors | `thiserror` (library/domain errors), `anyhow` (in `main`/scripts) |
| Validation | `validator` or `garde` |
| Config | `config` or `figment`, plus `dotenvy` |
| Logging/tracing | `tracing`, `tracing-subscriber`, optionally `opentelemetry` |
| Auth | `argon2`, `jsonwebtoken` |
| HTTP client | `reqwest` |
| OpenAPI | `utoipa` + `utoipa-swagger-ui` (or `aide`) |
| Testing | `tower::ServiceExt::oneshot`, `axum-test`, `testcontainers`, `wiremock` |

> ⚠️ Data in this section requires human validation.

---

## 2. Shared Architecture (use for every project)

```text
src/
├── main.rs          # load config, init tracing, build state, serve, graceful shutdown
├── lib.rs           # pub fn app(state) -> Router   (so tests can build the app)
├── config.rs        # typed Config struct loaded from env
├── error.rs         # AppError enum + impl IntoResponse
├── state.rs         # AppState { db, config, ... } (Clone, cheap: Arc / Pool)
├── routes/          # one module per resource: builds a Router, holds the handlers
│   └── todos.rs
├── domain/          # plain structs + business rules, no axum/sqlx imports
│   └── todo.rs
└── repo/            # database access (sqlx queries), returns domain types
    └── todo_repo.rs
tests/
└── api.rs           # black-box tests through the Router
migrations/          # sqlx migrations
```

Request flow:

```text
HTTP → tower layers (trace, cors, timeout, auth) → Router → handler
     → extractors (Path, Query, Json, State, AuthUser)
     → service/domain logic → repo (sqlx) → Postgres
     ← Result<Json<T>, AppError>  →  IntoResponse  → HTTP
```

### Patterns to learn once and reuse everywhere

**One error type that becomes an HTTP response:**

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("not found")]          NotFound,
    #[error("validation: {0}")]    Validation(String),
    #[error("unauthorized")]       Unauthorized,
    #[error(transparent)]          Db(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::NotFound      => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
            AppError::Unauthorized  => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Db(e) => {
                tracing::error!(error = ?e, "db error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
```

**Testable app:** `lib.rs` exposes `app(state) -> Router`. Tests call `app.oneshot(request)`, which needs no network port.

**Graceful shutdown:** `axum::serve(listener, app).with_graceful_shutdown(shutdown_signal())`.

> ⚠️ Data in this section requires human validation.

---

## 3. The Projects

### Project 1 — URL Shortener

**Goal:** practise Axum basics before adding a database.

**Requirements**
- `POST /links` with `{ "url": "https://..." }` returns `201 { "code": "aZ3k9Q", "short_url": "..." }`
- `GET /{code}` returns `307` with a redirect to the original URL, or `404`
- `GET /links/{code}/stats` returns `{ "url", "hits", "created_at" }`
- Reject invalid URLs with `422`
- Optional custom alias: `{ "url", "alias": "my-link" }`, returning `409` if the alias is taken
- Store data in memory: `Arc<RwLock<HashMap<String, Link>>>`

**Architecture:** a single `AppState` holding the map. Routes: `routes/links.rs`.

**Hints**
- Validate URLs with the `url` crate (`Url::parse`).
- Generate codes with `rand` + base62, or the `nanoid` crate.
- Use `axum::response::Redirect::temporary`.
- Hold a `std::sync::RwLock` only briefly and never across an `.await`. `tokio::sync::RwLock` is only needed when you must await while holding the lock.
- Increment `hits` atomically (`AtomicU64` inside `Link`, or take a write lock).

**Stretch:** expiry (`expires_at`), QR code endpoint (`qrcode` crate), move storage to SQLite with `sqlx`.

---

### Project 2 — Todo API v2 with Postgres

**Goal:** add real persistence to your existing `rust-api` Todo project.

**Requirements**
- Same endpoints as now, backed by Postgres
- Migrations in `migrations/` run on startup (`sqlx::migrate!()`)
- `GET /todos?completed=true&limit=20&offset=0&sort=-created_at`
- `created_at` / `updated_at` timestamps (`chrono` or `time`)
- Consistent JSON error body via `AppError`
- Config from env: `DATABASE_URL`, `PORT`, `RUST_LOG`

**Architecture:** `routes → repo → PgPool`. `AppState { pool: PgPool }`.

**Hints**
- `sqlx::query_as!` checks your SQL against the database at compile time. Run `cargo sqlx prepare` so builds work offline and in CI or Docker.
- Run Postgres locally with `docker run -e POSTGRES_PASSWORD=pw -p 5432:5432 postgres:17`.
- For integration tests, use `#[sqlx::test]`, which gives each test a fresh database.
- Use `Query<ListParams>` with `#[serde(default)]` for optional query parameters.
- Map `sqlx::Error::RowNotFound` to `AppError::NotFound`.

**Stretch:** cursor-based pagination, soft delete, `ETag` / `If-None-Match`.

---

### Project 3 — Auth Service

**Goal:** authentication, security, and custom extractors.

**Requirements**
- `POST /auth/register` with `{ email, password }`: hash the password and return `201`
- `POST /auth/login` returns `{ access_token, refresh_token }`
- `POST /auth/refresh` exchanges a refresh token for a new pair (rotation)
- `POST /auth/logout` revokes the refresh token
- `GET /me` is protected and returns the current user
- Roles: `user` and `admin`; `GET /admin/users` is admin-only

**Architecture**

```text
routes/auth.rs   → services/auth.rs (hash, verify, issue tokens) → repo/users.rs
extractors/auth_user.rs  → impl FromRequestParts for AuthUser
```

**Hints**
- Hash with `argon2` (`Argon2::default()`). Never store or log plain-text passwords.
- Keep access tokens short-lived (about 15 minutes, JWT via `jsonwebtoken`). Make refresh tokens long-lived, store them **hashed** in the database, and rotate them on every use.
- Implement `FromRequestParts` for `AuthUser`. Any handler that takes `AuthUser` as a parameter is then protected.
- For roles, write a `RequireAdmin(AuthUser)` extractor, or add a `middleware::from_fn` layer on a nested router.
- Return the same error for "unknown email" and "wrong password" so attackers can't tell which emails exist.

**Stretch:** email verification tokens, password reset, rate-limit login attempts, OAuth2 login with the `oauth2` crate.

---

### Project 4 — Rate-Limiting Reverse Proxy

**Goal:** understand `tower` middleware by writing your own.

**Requirements**
- Forward `/*path` to a configured upstream (for example `https://httpbin.org`)
- Per-client rate limit by IP or `X-API-Key`, such as 60 requests/minute (token bucket). Return `429` with a `Retry-After` header when exceeded.
- Add a request ID header (`x-request-id`) and log latency
- Upstream timeout returns `504`
- `GET /metrics` returns request counts per status code

**Architecture:** `Router` → `TraceLayer` → custom `RateLimitLayer` → `TimeoutLayer` → proxy handler that uses `reqwest::Client`.

**Hints**
- Start with `axum::middleware::from_fn_with_state`, which is easy. Then rewrite it as a real `tower::Layer` + `Service` to learn `poll_ready` and `call`.
- `tower-http` provides `SetRequestIdLayer`, `PropagateRequestIdLayer`, `TraceLayer`, and `TimeoutLayer`.
- Store token buckets in `DashMap<String, Bucket>`, or use the `governor` crate after you have built your own.
- Get client IPs with `ConnectInfo<SocketAddr>` (serve with `into_make_service_with_connect_info`).
- Stream bodies instead of buffering them (`reqwest` → `Body::from_stream`).

**Stretch:** Prometheus metrics (`metrics` + `metrics-exporter-prometheus`), circuit breaker, distributed limits in Redis.

---

### Project 5 — File Upload & Thumbnail Service

**Goal:** large request bodies, streaming, and CPU-bound work.

**Requirements**
- `POST /images` (multipart): accept PNG/JPEG up to 10 MB, store it, return `{ id }`
- `GET /images/{id}` streams the original
- `GET /images/{id}/thumb?w=200` resizes on first request, caches the result, and streams it
- Check the real file type from the magic bytes (don't trust `Content-Type`)
- `DELETE /images/{id}`

**Architecture:** `routes/images.rs` → `storage` trait (`LocalFs` now, `S3` later) + `repo/images.rs` for metadata.

**Hints**
- `axum::extract::Multipart`, plus `DefaultBodyLimit::max(10 * 1024 * 1024)`.
- Write uploads to disk chunk by chunk with `tokio::fs::File` + `AsyncWriteExt`.
- Image resizing is CPU-bound. Wrap it in `tokio::task::spawn_blocking`, or it blocks the runtime.
- Serve files with `tokio_util::io::ReaderStream` → `Body::from_stream`, or `tower_http::services::ServeDir`.
- Detect file types with the `infer` crate. Resize with the `image` crate.

**Stretch:** S3/MinIO backend (`aws-sdk-s3` or `object_store`), signed URLs, `Range` request support.

---

### Project 6 — Real-Time Chat

**Goal:** WebSockets, concurrency, and channels.

**Requirements**
- `POST /rooms`, `GET /rooms`
- `GET /rooms/{id}/ws` upgrades to a WebSocket. Messages are broadcast to everyone in the room.
- Persist messages. `GET /rooms/{id}/messages?before=<id>&limit=50` returns history.
- Presence: "user joined" / "user left" events
- Authenticate the WebSocket with the JWT from Project 3 (query parameter or first message)

**Architecture:** `AppState { rooms: DashMap<RoomId, broadcast::Sender<Event>>, pool }`. Each socket is split into a read task and a write task.

**Hints**
- `WebSocketUpgrade` → `ws.on_upgrade(|socket| handle(socket, state))`.
- Call `socket.split()` (from `futures::StreamExt`) and run the send and receive loops in `tokio::select!`. When one loop ends, abort the other.
- Use `tokio::sync::broadcast` for fan-out. Handle `RecvError::Lagged` when a client falls behind.
- Use a typed message protocol: `#[serde(tag = "type")] enum ClientMsg { Send { text }, Typing }`.
- Test with `websocat`, or write a test client with `tokio-tungstenite`.

**Stretch:** horizontal scaling with Redis pub/sub, typing indicators, Server-Sent Events alternative (`axum::response::Sse`).

---

### Project 7 — Background Job Queue

**Goal:** async workers, reliability, and shutdown.

**Requirements**
- `POST /jobs` with `{ "type": "send_email", "payload": {...}, "run_at"?: ... }` returns `202 { id }`
- `GET /jobs/{id}` returns status: `queued | running | succeeded | failed`, plus attempt count and last error
- N worker tasks claim jobs from Postgres, run them, and retry with exponential backoff (up to 5 attempts)
- Dead-letter: failed jobs can be retried manually with `POST /jobs/{id}/retry`
- Graceful shutdown: stop claiming new jobs, let running jobs finish (with a timeout), then exit

**Architecture**

```text
main → spawn API server + spawn N workers  (share PgPool, CancellationToken)
worker loop: claim → dispatch by type (trait JobHandler) → mark done / schedule retry
```

**Hints**
- Claim jobs atomically: `UPDATE jobs SET status='running' WHERE id = (SELECT id FROM jobs WHERE status='queued' AND run_at <= now() ORDER BY run_at FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING *`.
- Use `tokio_util::sync::CancellationToken` for shutdown and a `JoinSet` to wait for workers.
- Use Postgres `LISTEN/NOTIFY` (`sqlx::postgres::PgListener`) to wake idle workers instead of polling in a tight loop.
- Make handlers idempotent. A job can run twice if a worker crashes partway through.

**Stretch:** cron-style recurring jobs, a job priority column, per-type concurrency limits (`Semaphore`).

---

### Project 8 — Capstone: Multi-User Expense Tracker

**Goal:** combine the earlier projects into a production-style service.

**Requirements**
- Users and auth (reuse Project 3)
- Resources: `accounts`, `categories`, `transactions`, and `budgets`, each scoped to its owner (multi-tenant)
- `GET /reports/monthly?year=2026&month=11` returns totals per category plus budget vs. actual (SQL aggregation)
- CSV import of transactions (`POST /transactions/import`) run as a background job (reuse Project 7)
- OpenAPI spec + Swagger UI at `/docs`
- Health checks: `/health/live`, `/health/ready` (checks the database)
- Structured JSON logs and request tracing
- `Dockerfile` (multi-stage) + `docker-compose.yml` (app + Postgres)
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and `cargo sqlx prepare --check`

**Architecture:** the full layered layout from Section 2, plus a `services/` layer between routes and repos for business rules (for example "a transaction cannot use another user's account").

**Hints**
- **Authorization bugs are the main risk.** Add `WHERE user_id = $1` to every query. Write tests where user A tries to read user B's data and must get `404`.
- Store money as integer minor units (`i64` cents) or `rust_decimal`. Never use `f64`.
- Use `utoipa` derives on DTOs and handlers. Keep DTOs separate from database rows.
- Use the `csv` crate with `serde` for imports. Validate each row and report errors with row numbers.
- Use `testcontainers` to run real Postgres in CI tests.
- Use `cargo-chef` in the Dockerfile to cache dependency builds.

**Stretch:** deploy to Fly.io / a k8s cluster, OpenTelemetry traces to Jaeger, load test with `oha` or k6.

> ⚠️ Data in this section requires human validation.

---

## 4. General Hints & Habits

1. **Read the Axum examples folder** in the GitHub repo. It has a small example for almost every feature.
2. **Use `#[axum::debug_handler]`** when a handler won't compile. It turns long trait-bound errors into readable ones.
3. **Order extractors correctly.** Body-consuming extractors (`Json`, `Multipart`, `String`) must be the **last** handler argument.
4. **Keep the domain layer free of axum and sqlx.** Plain structs and functions are easy to unit test.
5. **Run `cargo clippy` and `cargo fmt` all the time.** Add `cargo watch -x test` (or `bacon`) for fast feedback.
6. **Don't block the runtime.** CPU-heavy work or sync I/O goes in `spawn_blocking`.
7. **Clone `AppState` cheaply.** Wrap its fields in `Arc` (`PgPool` is already an `Arc` internally).
8. **Test through the Router** with `oneshot`. Use real Postgres via `#[sqlx::test]` or `testcontainers` rather than mocking the database.
9. **Recommended reading:** *Zero To Production In Rust* (Luca Palmieri, written for actix-web, but the ideas carry over directly) and the *Async Rust* chapter of the Tokio tutorial.

> ⚠️ Data in this section requires human validation.

---

## References

Content is based on user request and Claude's general knowledge; the existing project files `Cargo.toml` and `Claude - Build and Run a Rust REST API from Scratch.md` in this repository were consulted for context. Validate crate versions and recommendations independently.

1. External: Axum repository and examples — https://github.com/tokio-rs/axum
2. External: Tokio tutorial — https://tokio.rs/tokio/tutorial
3. External: tower-http documentation — https://docs.rs/tower-http
4. External: SQLx repository — https://github.com/launchbadge/sqlx
5. External: Actix Web — https://actix.rs
6. External: Rocket — https://rocket.rs
7. External: Loco — https://loco.rs
8. External: utoipa (OpenAPI) — https://github.com/juhaku/utoipa
9. External: Zero To Production In Rust — https://www.zero2prod.com
10. Local file: `Cargo.toml` — rust-api repository, not publicly accessible

## Version History

| Version | Date | Author(s) | Changes |
|---------|------|-----------|---------|
| v0.1 | 10 November 2026 | Lam Pham | Initial draft — generated with Claude AI |

---

*Generated with Claude AI (qt-document-annotation) — validate all figures before use.*
