---
status: active
severity: medium
surfaces: [developer, security, ci]
adopted: 2026-05-05
adopted-because: pre-UNK-70 (sqlx-cli could not reach workspace shared-postgres) and pre-UNK-97 (no per-test DB pattern); both Done; recognised as debt 2026-05-05
lift-when-class: internal-refactor
lift-when: UNK-167 (sqlx query!/query_as! macros migration) merged to main
lifted: ~
superseded-by: ~
---

# Runtime sqlx queries instead of compile-time macros

## Constraint

When the backend was scaffolded, `sqlx-cli` could not connect to the
workspace's shared postgres ([UNK-70](https://linear.app/unkos/issue/UNK-70))
and there was no per-test DB pattern in place ([UNK-97](https://linear.app/unkos/issue/UNK-97)).
That made compile-time-checked queries (`query!`, `query_as!`,
`query_scalar!`) hard to use cleanly: the macros need a live DB at
compile time, and one wasn't reliably available.

Both blockers have since been resolved: UNK-70 is Done, and UNK-97
landed `#[sqlx::test]` providing per-test isolated databases.

## Workaround

All sqlx usage in `backend/src/` uses the runtime function form
(`sqlx::query(...)`, `sqlx::query_as(...)`, `sqlx::query_scalar(...)`)
instead of the macro form. The runtime functions do not validate
against a live DB at compile time. Type binding is done by-hand at the
call site.

Example sites: `backend/src/db.rs:28`, `backend/src/db.rs:49`,
`backend/src/db.rs:65`, `backend/src/test_support.rs:208,234,315`,
plus more in `routes/` / `services/` / `models/` (full count to be
inventoried during the lift work).

`backend/CLAUDE.md:25` claims "sqlx with compile-time checked queries"
— stale doc claim; this entry surfaces the mismatch.

## Why this isn't the right shape

Compile-time macros catch:

1. SQL syntax errors at compile, not runtime in production
2. Column-name typos at compile
3. Type mismatches between SQL and Rust at compile
4. Schema-evolution drift (a column rename in a migration without
   updating the query → CI red, not a 500 in production)
5. NULL-handling enforced (drop NOT NULL → result type changes → CI
   red until handled)

Runtime queries trade all of that for "no DB needed at compile time"
— a constraint that no longer applies. For a security-sensitive
open-source release where deploy-time SQL errors equal user-visible
breakage, the trade is wrong.

[UNK-108](https://linear.app/unkos/issue/UNK-108) (enum drift caught
only because that one class moved to compile-time sqlx derives) is
direct evidence the pattern catches real bugs.

## Lift conditions

[UNK-167](https://linear.app/unkos/issue/UNK-167) — adopt
`query!` / `query_as!` / `query_scalar!` macros across data-path
queries. Carve-outs documented for legitimate runtime use (DDL,
dynamic SQL, `set_config(...)` config calls).

When that PR (or PR series) merges:

1. Flip this entry to `status: lifted`, set `lifted: <date>`, set
   `superseded-by: <PR url>`.
2. Update `backend/CLAUDE.md` to reflect actual state with carve-outs
   documented.
3. [UNK-161](https://linear.app/unkos/issue/UNK-161) (operational
   follow-up: commit `.sqlx/` cache + `SQLX_OFFLINE=true` in builds +
   CI drift guard) becomes unblocked.

## Related

- [UNK-167](https://linear.app/unkos/issue/UNK-167) — the migration
  ticket (lift trigger)
- [UNK-161](https://linear.app/unkos/issue/UNK-161) — operational
  follow-up (offline cache + drift guard)
- [UNK-70](https://linear.app/unkos/issue/UNK-70) — sqlx-cli
  connectivity (resolved; was an original blocker)
- [UNK-97](https://linear.app/unkos/issue/UNK-97) — per-test DB
  pattern (resolved; was the other original blocker)
- [UNK-108](https://linear.app/unkos/issue/UNK-108) — enum drift
  prevented by compile-time sqlx derives (precedent within the
  project)
- `backend/CLAUDE.md` — stale claim to be updated as part of UNK-167
