---
status: lifted
severity: medium
surfaces: [developer, security, ci]
adopted: 2026-05-05
adopted-because: pre-UNK-70 (sqlx-cli could not reach workspace shared-postgres) and pre-UNK-97 (no per-test DB pattern); both Done; recognised as debt 2026-05-05
lift-when-class: internal-refactor
lift-when: UNK-167 (sqlx query!/query_as! macros migration) merged to main
lifted: 2026-05-06
superseded-by: https://github.com/unkos-dev/reverie/pull/157, https://github.com/unkos-dev/reverie/pull/158, https://github.com/unkos-dev/reverie/pull/159, https://github.com/unkos-dev/reverie/pull/160, https://github.com/unkos-dev/reverie/pull/161, https://github.com/unkos-dev/reverie/pull/162, https://github.com/unkos-dev/reverie/pull/163
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

All sqlx usage in `backend/src/` except `routes/health.rs`
(PR #157, 2026-05-05), `models/` (PR #158, 2026-05-05), the
enrichment service (PR #159, 2026-05-06), and `routes/` (PR #161,
2026-05-06) uses the runtime function form (`sqlx::query(...)`,
`sqlx::query_as(...)`, `sqlx::query_scalar(...)`) instead of the
macro form. The runtime functions do not validate against a live DB
at compile time. Type binding is done by-hand at the call site.

Initial inventory: 28 files, ~294 invocations (heaviest hitters:
`services/enrichment/orchestrator.rs` 41, `services/writeback/queue.rs`
37, `models/work.rs` 32, `services/writeback/orchestrator.rs` 28).
The remaining migration is grouped by module boundary as a PR series
following the bootstrap.

Until 2026-05-05, `backend/CLAUDE.md` described only the aspirational
posture ("sqlx with compile-time checked queries"); the wording was
corrected in the UNK-167 bootstrap PR to reflect the actual mid-
migration state with carve-outs documented.

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
dynamic SQL, `set_config(...)` config calls, enum-drift test probes).

## Lifted 2026-05-06

PR series #157–#162 migrated every data-path `sqlx::query(...)` to
the macro form. The closer PR (this debt-flip + UNK-172 grep-guard)
adds CI enforcement at `.github/sqlx-runtime-allowlist.txt`: new
runtime invocations outside the documented carve-out registry fail
CI. `backend/CLAUDE.md` now points at the allowlist as canonical
carve-out registry instead of this debt entry.

[UNK-161](https://linear.app/unkos/issue/UNK-161) (operational
follow-up: commit `.sqlx/` cache + `SQLX_OFFLINE=true` in builds + CI
drift guard) shipped in the bootstrap PR (#157).

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
