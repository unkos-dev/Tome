---
status: lifted
severity: medium
surfaces: [server-operator, end-user, developer]
adopted: 2026-05-05
adopted-because: tower-sessions-sqlx-store not wired when scaffold first introduced session middleware; recognised as debt 2026-05-05
lift-when-class: internal-refactor
lift-when: UNK-163 (PostgresStore swap) merged to main
lifted: 2026-05-07
superseded-by: PR #180 (UNK-163)
---

# MemoryStore for production sessions

## Constraint

When session middleware was first wired into the backend, the
project did not yet have `tower-sessions-sqlx-store` as a dependency
and the database pool wiring needed for a `PostgresStore` was
incomplete. `MemoryStore` was the path of least resistance to get
session-cookie auth working end-to-end.

By Step 2 of the blueprint the database pool existed in `AppState`;
the swap to `PostgresStore` was technically possible from that point
forward but was never triggered.

## Workaround

`backend/src/main.rs:36` builds the production session layer with
`MemoryStore::default()`. The inline comment at `main.rs:32-35`
acknowledges the limitation:

> NOTE: MemoryStore does not evict expired sessions server-side — the
> cookie ttl is the only bound. Swap to tower-sessions-sqlx-store if
> memory growth under sustained use becomes an issue.

That note frames the problem as "memory growth under sustained use"
— the more immediate problem (and the one that surfaced during
[UNK-159](https://linear.app/unkos/issue/UNK-159) staging-runtime
planning) is **every container restart logs out every user**. LXC
restarts on deploy. CI/CD cycle = login → deploy → forced re-login.
Friction during the staging build-out itself, not just under
sustained production use.

## Why this isn't the right shape

`PostgresStore` (or any persistent session store) gives:

1. Sessions survive container / process restarts → no forced re-auth
   on deploy
2. Server-side eviction on logout (currently there is no way to
   invalidate a session before its cookie TTL — a bypass for "force
   logout all sessions")
3. Predictable memory growth (bounded by the sessions table, not by
   process uptime × login rate)
4. Auditable session set (operators can query active sessions)

`MemoryStore` gives one thing: simpler test setup. Tests that share
session state with the harness can keep using `MemoryStore` (test
code is exempt from this debt — it's a legitimate testing pattern,
not a production workaround).

## Lift conditions

[UNK-163](https://linear.app/unkos/issue/UNK-163) — swap production
`MemoryStore` for `PostgresStore` (from `tower-sessions-sqlx-store`).
Independent of [UNK-101](https://linear.app/unkos/issue/UNK-101)
(tower-sessions 0.14 → 0.15 version bump, blocked on axum-login
peer pin); `tower-sessions-sqlx-store` ships a 0.14-compatible
release. The storage backend swap does not require the version bump.

When that PR merges:

1. Flip this entry to `status: lifted`, set `lifted`, set
   `superseded-by`.
2. Update the `main.rs:32-35` comment to remove the swap-suggestion
   (no longer applicable) and document the new behaviour.
3. Verify session persistence across container restart in an
   integration test.

## Related

- [UNK-163](https://linear.app/unkos/issue/UNK-163) — the swap ticket
  (lift trigger)
- [UNK-101](https://linear.app/unkos/issue/UNK-101) — tower-sessions
  version bump (orthogonal; blocked on axum-login)
- [UNK-159](https://linear.app/unkos/issue/UNK-159) — staging runtime
  master ticket where the friction surfaced
- `backend/src/main.rs:32-36` — workaround site
