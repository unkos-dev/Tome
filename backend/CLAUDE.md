# Backend — Rust + Axum

## Dev Database

Start the dev postgres: `docker compose up -d` from the repo root.
Port 5433 (5432 is taken by the host's shared-postgres).

**Roles** (created by `docker/init-roles.sql` on first start):

| Role | Connection | Purpose |
|------|-----------|---------|
| `reverie` | `postgres://reverie:reverie@localhost:5433/reverie_dev` | Schema owner. Runs migrations. Never used at runtime. |
| `reverie_app` | `postgres://reverie_app:reverie_app@localhost:5433/reverie_dev` | Web application. RLS enforced. |
| `reverie_ingestion` | `postgres://reverie_ingestion:reverie_ingestion@localhost:5433/reverie_dev` | Background pipeline. Scoped RLS. |
| `reverie_readonly` | `postgres://reverie_readonly:reverie_readonly@localhost:5433/reverie_dev` | Debug/reporting. SELECT only. |

Run migrations as the schema owner:
`DATABASE_URL=postgres://reverie:reverie@localhost:5433/reverie_dev sqlx migrate run`

## Conventions

- **Error handling:** Use `thiserror` for library errors, `anyhow` for application
  errors. Axum handlers return `Result<impl IntoResponse, AppError>` where `AppError`
  implements `IntoResponse`.
- **Database:** `sqlx` with compile-time checked queries. Migrations in
  `backend/migrations/`.
- **Testing:** Use `axum-test` for integration tests. Unit tests live alongside the
  code in `#[cfg(test)]` modules.
- **Logging:** Use `tracing` with structured fields. Never `println!` or `eprintln!`.
- **Formatting:** `cargo fmt` is enforced by CI. Do not fight the formatter.
- **Linting:** `cargo clippy -- -D warnings` is enforced by CI. Fix warnings, don't
  suppress them with `#[allow(...)]` unless there's a documented reason.

## Project Structure (as it grows)

```text
backend/
├── Cargo.toml
├── migrations/          # sqlx migrations
├── src/
│   ├── main.rs          # Entrypoint, router assembly, server setup
│   ├── auth/            # Authentication subsystem
│   │   ├── backend.rs   # axum-login AuthnBackend (OIDC credentials)
│   │   ├── middleware.rs # CurrentUser extractor (session + Basic auth)
│   │   ├── oidc.rs      # OIDC client init and discovery
│   │   └── token.rs     # Device token generation and argon2 verification
│   ├── routes/          # Axum route handlers, grouped by domain
│   ├── models/          # Database models and queries
│   ├── services/        # Business logic
│   ├── config.rs        # Environment-based configuration
│   ├── state.rs         # AppState (shared across handlers)
│   └── error.rs         # AppError type
└── tests/               # Integration tests (if separate from unit tests)
```
