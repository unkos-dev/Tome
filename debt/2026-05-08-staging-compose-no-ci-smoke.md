---
status: lifted
severity: medium
surfaces: [server-operator, ci, security]
adopted: 2026-05-08
adopted-because: PR #179 review (CodeRabbit) flagged absence of automated coverage for staging compose init path; deferred to ship the compose seam itself
lift-when-class: internal-refactor
lift-when: UNK-185 (CI smoke test for staging compose) merged to main
lifted: 2026-05-11
superseded-by: PR #209
---

# Staging compose has no automated CI smoke test

## Constraint

`docker/compose.staging.yml` lands as a non-trivial bootstrap surface:

- `init-roles.sql` runs against a fresh data directory, reading three
  separate password env vars to provision runtime roles
  (`reverie_app`, `reverie_ingestion`, `reverie_readonly`).
- The postgres parent-dir mount fix (`/var/lib/postgresql` instead of
  `/var/lib/postgresql/data`) is a postgres:18+ requirement that has
  no test asserting "fresh volume → boots green → roles exist".
- `env_file` / `environment: ${VAR:?required}` interpolation has only
  manual validation against a populated `.env` stub.

Repo TDD policy ("no feature is complete without tests, including
negative cases") applies in spirit but landing a CI job that spins up
docker compose with privileged docker, manages ephemeral secrets, and
asserts on postgres role state is real engineering work — not in the
shape of a 10-line unit test.

## Workaround

PR #179 ships with manual local validation:

1. `docker compose -f docker/compose.staging.yml config` against a
   populated `.env` (declarative validation only; doesn't actually run).
2. Standalone postgres init smoke against env-supplied passwords —
   asserts `reverie_app` connects with the supplied password.
3. Standalone postgres init smoke against role-name fallback — asserts
   the dev workflow path stays unchanged.
4. Dev `docker compose down -v && up -d` against `docker-compose.yml`
   (the same parent-dir mount fix) — asserts sqlx migrations apply
   cleanly on a fresh volume.

These steps live in the PR review summary, not in CI. Future regressions
are caught only on the next manual run or on staging boot.

## Why this isn't the right shape

Two real failure modes are uncovered:

1. **Silent role-misconfiguration on staging deploys.** If
   `init-roles.sql` is edited and a role's `LOGIN PASSWORD` clause
   changes shape, the next staging deploy boots, postgres starts
   healthy, and the application fails with a `28P01` at runtime — _after_
   traffic has been flipped. CI catching this on PR is much cheaper.
2. **Required-env-var regression.** The `${VAR:?required}` interpolation
   only fires when a deploy actually runs. A typo in a key name in
   `.env.example` won't surface until someone copies it to `.env` on the
   LXC host. A negative-path test that runs compose with a deliberately
   missing var would catch this on the PR that introduced the typo.

`MemoryStore for production sessions`-style debt: the workaround works,
but the cost is paid by the operator (or the PR author) on each future
change instead of by CI.

## Lift conditions

[UNK-185](https://linear.app/unkos/issue/UNK-185) — CI job that:

1. Brings up `docker/compose.staging.yml` against a populated `.env`
   stub, waits for postgres healthcheck, asserts the three runtime roles
   exist with expected `CONNECT` grants, tears down cleanly.
2. Brings up the same compose with a deliberately-missing required env
   var, asserts the `:?` substitution surfaces a hard fail.

When that PR merges:

1. Flip this entry to `status: lifted`, set `lifted`, set
   `superseded-by`.
2. Remove the manual-validation steps from `docker/compose.staging.yml`
   review checklists; CI now owns the gate.

## Related

- [UNK-185](https://linear.app/unkos/issue/UNK-185) — the CI job
  ticket (lift trigger)
- [UNK-162](https://linear.app/unkos/issue/UNK-162) — parent ticket
  (staging compose itself)
- [UNK-159](https://linear.app/unkos/issue/UNK-159) — staging runtime
  master ticket
- PR #179 — landed the workaround (manual smoke only)
- `docker/compose.staging.yml`, `docker/init-roles.sql`,
  `docker/staging.env.bootstrap.example`,
  `docker/staging.env.runtime.example` — workaround surface
