---
status: active
severity: low
surfaces: [developer]
adopted: 2026-05-05
adopted-because: openidconnect v4 CoreIdTokenClaims::new public API requires chrono types at the call site; documented inline in backend/CLAUDE.md and test_support.rs at adoption time
lift-when-class: dep-unblocks
lift-when: openidconnect v5 stable release decouples chrono types, OR migrate to alternative OIDC lib, OR introduce a wrap-and-convert layer at the test boundary
lifted: ~
superseded-by: ~
---

# chrono in OIDC test mock

## Constraint

The project standard for date/time handling is the `time` crate, not
`chrono` (recorded in `backend/CLAUDE.md` and in memory
`project_time_not_chrono.md`). The standard was set after the project
scaffold predated the decision; the blueprint mentions chrono but the
ratified posture is `time`.

The OIDC test mock (`backend/src/test_support.rs::oidc_mock`) builds
ID-token claims via `openidconnect::core::CoreIdTokenClaims::new`.
That constructor's public API in openidconnect v4 takes chrono types
(`chrono::DateTime<Utc>`) for issued-at / expiration / not-before.
The types are non-negotiable at the call site.

## Workaround

`backend/Cargo.toml` includes `chrono` as a dependency in `dev-dependencies`
(or feature-gated, depending on current state). `oidc_mock` constructs
chrono `DateTime<Utc>` values for the duration of the mock setup.
No first-party code outside `oidc_mock` touches chrono.

`backend/CLAUDE.md` documents the carve-out explicitly:

> The single documented exception is `test_support.rs::oidc_mock`,
> where `openidconnect` v4's public API (`CoreIdTokenClaims::new`)
> forces chrono types on the call site. That use is contained to the
> OIDC mock and must not spread elsewhere.

## Why this isn't the right shape

Two crates for the same job is taxing for several reasons:

1. Cognitive overhead — contributors have to remember which crate
   applies where, and what conversions exist between them.
2. Compile time — chrono's deps add to the dev build.
3. Audit surface — chrono has its own CVE history; `time` was chosen
   in part for its smaller surface. Carrying chrono in dev-deps
   widens the attack surface against the test toolchain (relevant if
   tests are ever run against untrusted input, which they shouldn't
   be but the discipline matters).
4. The carve-out invites scope creep — every new test that touches
   OIDC claims has the same temptation.

## Lift conditions

Three independent paths can lift this debt:

1. **Upstream dep-unblock**: openidconnect v5 (or any future version)
   ships a constructor that takes generic time types or `time` crate
   types. Track the upstream issue tracker for openidconnect.
2. **Migrate OIDC lib**: switch to a different OIDC client crate that
   uses `time` natively. Substantial refactor — not motivated by this
   debt alone, but a future libauth refactor could absorb the change.
3. **Wrap-and-convert at the boundary**: write a thin local adapter
   (`oidc_mock::time_to_chrono`) that takes `time::OffsetDateTime` and
   returns the chrono type, contained to the mock. Lifts the
   "chrono touches first-party code" smell without removing the
   chrono dep. Cheaper than (1) or (2). Does not eliminate the
   dependency but isolates the conversion site to a single named
   function with a clear deletion target post-(1)/(2).

When any path completes:

1. Flip this entry to `status: lifted`, set `lifted`, set
   `superseded-by`.
2. Update `backend/CLAUDE.md` to remove the carve-out (or narrow it
   if path 3 is taken).
3. Remove chrono from `Cargo.toml` if path 1 or 2 is taken.

## Related

- `backend/CLAUDE.md` — carve-out documentation (would update on
  lift)
- `backend/src/test_support.rs::oidc_mock` — workaround site
- Memory: `project_time_not_chrono.md` — project posture
- No Linear ticket yet — file as part of any libauth refactor or
  when an upstream dep-unblock surfaces. Until then, this debt entry
  is the canonical record.
