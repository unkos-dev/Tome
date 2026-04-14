# Plan: Application Skeleton

## Summary

Wire the Axum server into a production-ready shape: configuration loading from
environment variables, database connection pool (SQLx + PgPool), structured logging
(tracing), graceful shutdown, and the `AppError` type that maps domain errors to HTTP
responses. Creates the module structure (`routes/`, `models/`, `services/`) that all
subsequent features plug into.

## User Story

As a developer building on Tome,
I want a well-structured application skeleton with config, DB pool, error handling, and health checks,
so that every subsequent feature plugs into an established, tested foundation.

## Problem -> Solution

**Current state:** Minimal `main.rs` with a hardcoded health endpoint, no DB connection,
no config, no error type, no module structure.

**Desired state:** Production-ready server skeleton with config loading, PgPool, RLS
helper, structured logging, graceful shutdown, `AppError`, module directories, and
health/readiness endpoints with integration tests.

## Metadata

- **Complexity**: Medium
- **Source PRD**: `/home/coder/Tome/plans/BLUEPRINT.md`
- **PRD Phase**: Step 2 — Application Skeleton
- **Estimated Files**: 8-10

---

## UX Design

N/A — internal change. No user-facing UX transformation.

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `backend/src/main.rs` | all | Current entrypoint to refactor |
| P0 | `backend/Cargo.toml` | all | Current dependencies |
| P0 | `backend/CLAUDE.md` | all | Conventions: error handling, structure, testing |
| P1 | `.env.example` | all | DATABASE_URL pattern and role separation |
| P1 | `backend/migrations/20260412150001_extensions_enums_and_roles.up.sql` | 1-18 | Enum types and role grants |
| P2 | `docker-compose.yml` | all | Postgres service on port 5433 |
| P2 | `.github/workflows/ci.yml` | all | CI has no postgres — affects test strategy |

---

## Patterns to Mirror

### NAMING_CONVENTION

```rust
// SOURCE: backend/Cargo.toml:1-2
// Crate: tome-api, edition 2024
// Modules: snake_case files, PascalCase types, SCREAMING_SNAKE_CASE constants
```

### ERROR_HANDLING

```rust
// SOURCE: backend/CLAUDE.md — Conventions section
// Use thiserror for library errors. Axum handlers return
// Result<impl IntoResponse, AppError> where AppError implements IntoResponse.
// Never leak internals in HTTP responses.
```

### LOGGING_PATTERN

```rust
// SOURCE: backend/src/main.rs:14-16
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();
// Use tracing with structured fields. Never println! or eprintln!.
```

### TEST_STRUCTURE

```rust
// SOURCE: backend/src/main.rs:26-38
// Unit tests in #[cfg(test)] modules alongside code.
// Integration tests use axum-test TestServer.
#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn health_returns_ok() {
        let server = TestServer::new(app());
        // ...
    }
}
```

### DB_CONNECTION

```text
// SOURCE: .env.example:6
// Runtime uses tome_app role (RLS enforced):
DATABASE_URL=postgres://tome_app:tome_app@localhost:5433/tome_dev
// Migrations use tome (schema owner):
DATABASE_URL=postgres://tome:tome@localhost:5433/tome_dev
```

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `backend/Cargo.toml` | UPDATE | Add thiserror, anyhow, tower-http, dotenvy deps |
| `backend/src/config.rs` | CREATE | Config struct loaded from env vars |
| `backend/src/error.rs` | CREATE | AppError enum with IntoResponse |
| `backend/src/db.rs` | CREATE | PgPool init and acquire_with_rls() |
| `backend/src/state.rs` | CREATE | AppState struct |
| `backend/src/routes/mod.rs` | CREATE | Route module re-exports |
| `backend/src/routes/health.rs` | CREATE | Health and readiness endpoints |
| `backend/src/models/mod.rs` | CREATE | Empty module placeholder |
| `backend/src/services/mod.rs` | CREATE | Empty module placeholder |
| `backend/src/main.rs` | UPDATE | Refactor to use config, pool, state, graceful shutdown |

## NOT Building

- OIDC authentication (Step 3)
- Any route handlers beyond health/readiness
- TOML config file loading (env vars only)
- Database models or queries beyond health ping
- Frontend changes

---

## Step-by-Step Tasks

### Task 1: Add dependencies to Cargo.toml

- **ACTION**: Add missing crates to `[dependencies]`
- **IMPLEMENT**: Add `thiserror = "2"`, `anyhow = "1"`, `tower-http = { version = "0.6", features = ["cors", "trace"] }`, `dotenvy = "0.15"`. Add `tls-rustls` feature to existing `sqlx` entry.
- **MIRROR**: Match existing dep style in Cargo.toml (inline features)
- **IMPORTS**: N/A
- **GOTCHA**: Blueprint says `chrono` but Cargo.toml already uses `time` with sqlx `time` feature. Do NOT add chrono — use `time` consistently. The `toml` crate is listed in the blueprint but not needed yet — skip to avoid unused deps.
- **VALIDATE**: `cargo check` compiles cleanly

### Task 2: Create `src/config.rs`

- **ACTION**: Create config module that loads from environment variables
- **IMPLEMENT**:
  ```rust
  pub struct Config {
      pub port: u16,               // TOME_PORT, default 3000
      pub database_url: String,    // DATABASE_URL, required
      pub library_path: String,    // TOME_LIBRARY_PATH, default "./library"
      pub ingestion_path: String,  // TOME_INGESTION_PATH, default "./ingestion"
      pub quarantine_path: String, // TOME_QUARANTINE_PATH, default "./quarantine"
      pub log_level: String,       // RUST_LOG, default "info"
  }
  
  impl Config {
      pub fn from_env() -> Result<Self, ConfigError> {
          // Load .env if present (dev only, dotenvy::dotenv().ok())
          // Read each var, apply defaults, validate database_url is present
      }
  }
  ```
  Define `ConfigError` with `thiserror` (missing required var).
- **MIRROR**: ERROR_HANDLING pattern — thiserror for typed errors
- **IMPORTS**: `std::env`, `thiserror`, `dotenvy`
- **GOTCHA**: `dotenvy::dotenv().ok()` — silently ignore missing `.env` (production won't have one). `database_url` has no default — fail fast if missing.
- **VALIDATE**: Unit test: set env vars, call `Config::from_env()`, assert fields

### Task 3: Create `src/error.rs`

- **ACTION**: Create `AppError` enum implementing Axum's `IntoResponse`
- **IMPLEMENT**:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum AppError {
      #[error("not found")]
      NotFound,
      #[error("unauthorized")]
      Unauthorized,
      #[error("validation error: {0}")]
      Validation(String),
      #[error(transparent)]
      Internal(#[from] anyhow::Error),
  }
  
  impl IntoResponse for AppError {
      fn into_response(self) -> Response {
          // Map to status code + JSON body { "error": message }
          // Internal: log with tracing::error!, return 500 generic message
          // Never expose internal details
      }
  }
  ```
- **MIRROR**: ERROR_HANDLING, LOGGING_PATTERN
- **IMPORTS**: `axum::response::{IntoResponse, Response}`, `axum::http::StatusCode`, `serde_json`, `tracing`
- **GOTCHA**: `Internal` variant logs the real error server-side but returns generic "internal server error" to the client. Do NOT leak the anyhow error chain in the response body.
- **VALIDATE**: Unit test: each variant maps to expected status code. Internal error does not leak details.

### Task 4: Create `src/db.rs`

- **ACTION**: Create database pool initialization and RLS helper
- **IMPLEMENT**:
  ```rust
  use sqlx::PgPool;
  use sqlx::postgres::PgPoolOptions;
  
  pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
      PgPoolOptions::new()
          .max_connections(10)
          .connect(database_url)
          .await
  }
  
  /// Acquire a connection with RLS context set for the given user.
  /// Uses set_config with is_local=true, scoped to the current transaction.
  pub async fn acquire_with_rls(
      pool: &PgPool,
      user_id: uuid::Uuid,
  ) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, sqlx::Error> {
      let mut tx = pool.begin().await?;
      sqlx::query("SELECT set_config('app.current_user_id', $1::text, true)")
          .bind(user_id.to_string())
          .execute(&mut *tx)
          .await?;
      Ok(tx)
  }
  ```
- **MIRROR**: DB_CONNECTION pattern (tome_app role, RLS via session vars)
- **IMPORTS**: `sqlx::{PgPool, postgres::PgPoolOptions}`, `uuid::Uuid`
- **GOTCHA**: `set_config(..., true)` means "local to current transaction" — equivalent to `SET LOCAL`. The third arg `true` = is_local. This matches the RLS policies in migration 7.
- **VALIDATE**: Integration test (requires running DB): acquire_with_rls sets the session variable, verify with `current_setting('app.current_user_id')`

### Task 5: Create `src/state.rs`

- **ACTION**: Create `AppState` struct that holds shared application state
- **IMPLEMENT**:
  ```rust
  use sqlx::PgPool;
  use crate::config::Config;
  
  #[derive(Clone)]
  pub struct AppState {
      pub pool: PgPool,
      pub config: Config,  // Config needs to derive Clone
  }
  ```
- **MIRROR**: NAMING_CONVENTION — PascalCase struct
- **IMPORTS**: `sqlx::PgPool`, `crate::config::Config`
- **GOTCHA**: `Config` must derive `Clone`. Keep `AppState` minimal — add fields in future steps.
- **VALIDATE**: Compiles. Tested via integration test.

### Task 6: Create module directories and placeholders

- **ACTION**: Create `src/routes/mod.rs`, `src/models/mod.rs`, `src/services/mod.rs`
- **IMPLEMENT**: `routes/mod.rs` re-exports `health` module. `models/mod.rs` and `services/mod.rs` are empty (with a doc comment).
- **MIRROR**: Module structure from backend/CLAUDE.md
- **IMPORTS**: N/A
- **GOTCHA**: Don't add dead code. Empty modules with doc comments are fine.
- **VALIDATE**: `cargo check` — no unused warnings

### Task 7: Move health endpoint to `src/routes/health.rs`

- **ACTION**: Extract health handler from main.rs, add readiness check
- **IMPLEMENT**:
  ```rust
  use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
  use crate::state::AppState;
  
  pub fn router() -> Router<AppState> {
      Router::new()
          .route("/health", get(health))
          .route("/health/ready", get(ready))
  }
  
  async fn health() -> &'static str {
      "ok"
  }
  
  async fn ready(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
      sqlx::query("SELECT 1")
          .execute(&state.pool)
          .await
          .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
      Ok("ok")
  }
  ```
- **MIRROR**: TEST_STRUCTURE — the existing health test pattern
- **IMPORTS**: `axum::{extract::State, Router, routing::get}`, `crate::state::AppState`
- **GOTCHA**: `/health` stays stateless (liveness). `/health/ready` checks DB (readiness). Returns 503 if DB is down, not an `AppError` — keep it simple.
- **VALIDATE**: Unit test for `/health` (no DB needed). Integration test for `/health/ready` (needs DB).

### Task 8: Refactor `src/main.rs`

- **ACTION**: Wire everything together: config, pool, state, middleware, graceful shutdown
- **IMPLEMENT**:
  ```rust
  mod config;
  mod db;
  mod error;
  mod models;
  mod routes;
  mod services;
  mod state;
  
  use crate::config::Config;
  use crate::state::AppState;
  
  pub fn build_router(state: AppState) -> Router {
      Router::new()
          .merge(routes::health::router())
          .layer(tower_http::trace::TraceLayer::new_for_http())
          .with_state(state)
  }
  
  #[tokio::main]
  async fn main() {
      let config = Config::from_env().expect("invalid configuration");
  
      tracing_subscriber::fmt()
          .with_env_filter(
              tracing_subscriber::EnvFilter::try_from_default_env()
                  .unwrap_or_else(|_| config.log_level.parse().unwrap())
          )
          .init();
  
      let pool = db::init_pool(&config.database_url)
          .await
          .expect("failed to connect to database");
  
      let state = AppState { pool, config };
      let app = build_router(state);
  
      let listener = tokio::net::TcpListener::bind(("0.0.0.0", 3000))
          .await
          .expect("failed to bind");
  
      tracing::info!("listening on {}", listener.local_addr().unwrap());
  
      axum::serve(listener, app)
          .with_graceful_shutdown(shutdown_signal())
          .await
          .expect("server error");
  }
  
  async fn shutdown_signal() {
      tokio::signal::ctrl_c().await.expect("failed to listen for ctrl-c");
      tracing::info!("shutdown signal received");
  }
  ```
- **MIRROR**: LOGGING_PATTERN, existing main.rs structure
- **IMPORTS**: All new modules
- **GOTCHA**: Bind to `config.port`, not hardcoded 3000. Expose `pub fn build_router(state: AppState) -> Router` for test use.
- **VALIDATE**: `cargo build`, manual start and curl

### Task 9: Write tests

- **ACTION**: Write unit tests and integration tests
- **IMPLEMENT**:
  - **Unit tests** (no DB required):
    - `config.rs`: test `Config::from_env()` with set/unset vars
    - `error.rs`: test each `AppError` variant maps to correct status code
    - `routes/health.rs`: test `/health` returns 200 "ok" (stateless, no DB needed)
  - **Integration tests** (require DB, marked `#[ignore]`):
    - `tests/health_test.rs`: start server with real pool, test `/health` and `/health/ready`
    - `db.rs`: test `acquire_with_rls()` sets session variable correctly
  - **Gate DB tests**: mark with `#[ignore]` since CI has no postgres service
- **MIRROR**: TEST_STRUCTURE — axum-test TestServer pattern
- **IMPORTS**: `axum_test::TestServer`, `sqlx::PgPool`
- **GOTCHA**: CI workflow has no postgres. Mark DB-dependent tests with `#[ignore]` and document how to run them locally. Non-DB tests must pass in CI with `cargo test`.
- **VALIDATE**: `cargo test` passes (non-DB tests). `cargo test -- --ignored` passes locally with DB.

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| `config_from_env_with_defaults` | Only DATABASE_URL set | Config with default port 3000, default paths | No |
| `config_from_env_all_vars` | All vars set | Config with custom values | No |
| `config_from_env_missing_database_url` | No DATABASE_URL | Error | Yes |
| `app_error_not_found_status` | `AppError::NotFound` | 404 | No |
| `app_error_unauthorized_status` | `AppError::Unauthorized` | 401 | No |
| `app_error_validation_status` | `AppError::Validation(...)` | 422 | No |
| `app_error_internal_status` | `AppError::Internal(...)` | 500 | No |
| `app_error_internal_no_leak` | `AppError::Internal(...)` | Body has no internal details | Yes |
| `health_returns_ok` | GET /health | 200 "ok" | No |

### Integration Tests (DB required, #[ignore])

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| `health_ready_with_db` | GET /health/ready (DB up) | 200 "ok" | No |
| `acquire_with_rls_sets_user` | Call with user_id | Session var matches | No |

### Edge Cases Checklist

- [x] Missing required config (DATABASE_URL) -> clear error
- [x] DB unavailable -> /health/ready returns 503
- [x] AppError::Internal does not leak stack traces
- [ ] Invalid DATABASE_URL format (sqlx handles this)
- [ ] Port already in use (tokio bind error — acceptable panic at startup)

---

## Validation Commands

### Static Analysis

```bash
cd backend && cargo fmt --check
```

EXPECT: Zero formatting issues

```bash
cd backend && cargo clippy -- -D warnings
```

EXPECT: Zero warnings

### Unit Tests

```bash
cd backend && cargo test
```

EXPECT: All non-ignored tests pass

### Integration Tests (local only)

```bash
cd backend && DATABASE_URL=postgres://tome_app:tome_app@localhost:5433/tome_dev cargo test -- --ignored
```

EXPECT: All tests pass (requires `docker compose up -d` and migrations run)

### Build

```bash
cd backend && cargo build
```

EXPECT: Clean build

### Docker Build

```bash
docker build -t tome:dev .
```

EXPECT: Image builds successfully

### Manual Validation

- [ ] `docker compose up -d` starts postgres
- [ ] Run migrations: `DATABASE_URL=postgres://tome:tome@localhost:5433/tome_dev sqlx migrate run`
- [ ] Start server: `cd backend && cargo run`
- [ ] `curl http://localhost:3000/health` returns `ok`
- [ ] `curl http://localhost:3000/health/ready` returns `ok`
- [ ] Stop postgres, `/health/ready` returns 503, `/health` still returns 200
- [ ] Send SIGTERM/Ctrl-C, server shuts down gracefully (logs shutdown message)

---

## Acceptance Criteria

- [ ] All tasks completed
- [ ] All validation commands pass
- [ ] Tests written and passing
- [ ] No type errors
- [ ] No lint errors
- [ ] `AppError` maps NotFound->404, Unauthorized->401, Validation->422, Internal->500
- [ ] Internal errors never leak details to HTTP responses
- [ ] `acquire_with_rls()` correctly sets `app.current_user_id` session variable
- [ ] Graceful shutdown on SIGTERM/SIGINT
- [ ] Health endpoint (liveness) and readiness endpoint (DB ping) work

## Completion Checklist

- [ ] Code follows discovered patterns
- [ ] Error handling matches codebase style (thiserror + IntoResponse)
- [ ] Logging uses tracing with structured fields
- [ ] Tests follow axum-test pattern
- [ ] No hardcoded values (config from env)
- [ ] No unnecessary scope additions
- [ ] Self-contained — no questions needed during implementation

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| CI has no postgres — DB tests fail in CI | High | Medium | Gate DB tests with `#[ignore]`, document local-only. Add CI postgres in a follow-up PR. |
| sqlx compile-time query checking not yet wired | Low | Low | No checked queries in this step. `acquire_with_rls` uses raw `sqlx::query()` which is fine. |
| `set_config` RLS approach may differ from what Step 1 migrations expect | Low | High | Verified: migration 7 creates RLS policies using `current_setting('app.current_user_id')`. Matches. |

## Notes

- **`time` not `chrono`**: Blueprint Step 2 mentions `chrono` but Cargo.toml already uses `time` and sqlx has the `time` feature. Sticking with `time` to avoid dual-time-library.
- **`toml` dep deferred**: Blueprint lists it but no TOML file loading is needed. Adding unused deps invites warnings. Will add when actually needed.
- **`anyhow` dep**: Needed for `AppError::Internal(#[from] anyhow::Error)`. Added to Cargo.toml.
- **`app()` function**: Current main.rs exposes `pub fn app() -> Router` for testing. Refactor to `pub fn build_router(state: AppState) -> Router` so tests can inject state.
