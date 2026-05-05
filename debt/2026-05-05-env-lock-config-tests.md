---
status: active
severity: high
surfaces: [developer, security, ci]
adopted: 2026-05-05
adopted-because: Config::from_env reads process-global env vars; tests had no clean way to vary input without mutating globals; recognised as debt 2026-05-05
lift-when-class: internal-refactor
lift-when: UNK-100 (Config::from_env takes env source as parameter) merged to main
lifted: ~
superseded-by: ~
---

# ENV_LOCK + unsafe env mutation in config tests

## Constraint

`Config::from_env` reads its inputs from process-global environment
variables (`std::env::var(...)`). Tests need to vary these inputs to
exercise different config shapes (missing required vars, malformed
values, optional combinations). Without an architectural seam letting
tests inject env values, the only available path was mutating the
process env from inside tests.

Cargo runs tests in parallel within a binary. Mutating a process
global from one test affects every concurrently-running test that
reads the same global. `Config::from_env` is not the only reader —
the `#[sqlx::test]` macro also reads `DATABASE_URL` at runtime to set
up its per-test database.

To prevent collisions among config tests, an `ENV_LOCK` mutex was
introduced in `backend/src/test_support.rs` and serialised acquisition
was added around every `with_env(...)` call site. The mutex
serialises mutations among tests that opt into it; it does not
serialise against `sqlx::test`'s reads.

## Workaround

`backend/src/test_support.rs::ENV_LOCK` is a process-wide mutex
acquired by every `with_env(...)` invocation in `backend/src/config.rs`
(visible at `config.rs:511-540`, `config.rs:920-980`, etc.).

Inside the held lock, env vars are mutated via `std::env::set_var` /
`std::env::remove_var` — both `unsafe` operations in the version of
Rust the project uses. SAFETY comments justify each block on the
basis of "ENV_LOCK held". The justification is partial: it covers
mutator-vs-mutator races, not mutator-vs-`sqlx::test`-reader races.

PR #25 surgically normalised every literal `DATABASE_URL` in
`with_env(...)` calls to match CI's host + DB so the `sqlx::test`
master-pool invariant doesn't fire. That fixed 12 of 13 racy cases.
One residual (`from_env_missing_database_url`) clears the env var
entirely, and no URL normalisation can help.

## Why this isn't the right shape

The fundamental problem is `Config::from_env` reading from a global.
The architectural fix exists and is documented in
[UNK-100](https://linear.app/unkos/issue/UNK-100):

```rust
fn from_source(get: &dyn Fn(&str) -> Option<String>) -> Result<Self, ConfigError>
```

Tests pass a HashMap-backed closure; production passes
`std::env::var(...).ok()`. Eliminates env mutation entirely. Removes
the need for `ENV_LOCK`. Removes every `unsafe` block. Removes the
race that PR #25 only partially closed.

The reason this debt is `severity: high` rather than `medium`:

1. Uses `unsafe` in non-trivial test code. Adds cognitive load and
   audit surface.
2. Has a known residual race that the surgical fix could not address.
3. The cleaner pattern (env injection) is a generally good practice
   for any code that reads environment — applying it here teaches the
   pattern for everywhere else.
4. `backend/CLAUDE.md` rule: "No `unwrap()` or `expect()` in non-test
   code" — the spirit is "don't use unsafe shortcuts". Tests should
   not be the exception that proves the pattern.

## Lift conditions

[UNK-100](https://linear.app/unkos/issue/UNK-100) — refactor
`Config::from_env` to take an env source as parameter. Migrate every
`with_env(...)` test site to use the injected source. Delete
`with_env` and `ENV_LOCK` entirely from `test_support.rs`. Promoted
from Low to High priority on 2026-05-05 under the new tracked-debt
posture.

When that PR merges:

1. Flip this entry to `status: lifted`, set `lifted`, set
   `superseded-by`.
2. Verify no remaining `std::env::set_var` / `std::env::remove_var`
   in `backend/src/`.

## Related

- [UNK-100](https://linear.app/unkos/issue/UNK-100) — the refactor
  ticket (lift trigger)
- [UNK-97](https://linear.app/unkos/issue/UNK-97) — per-test DB
  pattern via `#[sqlx::test]` (the architectural seam this debt
  predates)
- [UNK-102](https://linear.app/unkos/issue/UNK-102) — duplicate of
  UNK-100, kept as historical record
- `backend/src/test_support.rs::ENV_LOCK` — workaround site
- `backend/src/config.rs:511-540, 920-980` — mutation call sites
