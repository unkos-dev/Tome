# Implementation Report

**Plan**: `.claude/PRPs/plans/metadata-writeback.plan.md`
**Source PRD**: `plans/BLUEPRINT.md` Step 8 (lines 929–1281)
**Branch**: `feat/metadata-writeback`
**Date**: 2026-04-19
**Status**: COMPLETE

---

## Summary

Implemented BLUEPRINT Step 8: metadata writeback to managed EPUBs. Every
canonical pointer move (from the enrichment orchestrator's `Decision::Apply`
arm and from the accept / revert routes in `routes/metadata`) now enqueues
a `writeback_jobs` row in the same transaction. A background worker drains
the queue, rewrites the OPF via an event-stream `quick-xml` transform,
optionally embeds a new cover, repacks the EPUB (preserving per-entry
compression + mimetype-first constraint), swaps the file atomically, and
updates `manifestations.current_file_hash` on success. `ingestion_file_hash`
stays immutable as the audit trail.

---

## Assessment vs Reality

| Metric     | Predicted   | Actual   | Reasoning                                                                      |
| ---------- | ----------- | -------- | ------------------------------------------------------------------------------ |
| Complexity | Large (~1500–2000 LOC across Rust + SQL + tests) | Close — ~1600 LOC net added | OPF rewriter + cover plan + queue + orchestrator all landed with test coverage. |
| Confidence | Strongest (file mutation + concurrent-write race + crash recovery + cover handling) | Matched | Adversarial-review spec in BLUEPRINT was comprehensive; only one open gap (end-to-end rollback test — see Deviations). |

### Deviations from plan

1. **Grant widened from INSERT-only to SELECT+INSERT on `reverie_ingestion`.**
   Advisor flagged that Task 3a emits jobs from inside the enrichment
   orchestrator's tx, which runs on the ingestion pool. `INSERT ... RETURNING`
   needs SELECT on the returned columns. Comment in the migration narrowly
   scopes the "ingestion never writes back to managed files" invariant to
   file-mutation, not job-emission bookkeeping.

2. **Added RLS system-context policies on `manifestations`.** The worker
   runs on `reverie_app` (per plan Task 10 GOTCHA) but has no user context;
   the existing RLS policies filter out every row. Added
   `manifestations_select_system` + `manifestations_update_system` policies
   gated on `app.current_user_id` being unset — user-facing handlers still
   hit the user policies because they SET LOCAL that variable.

3. **Task 21 (post-validation rollback) integration test deferred.** ~~The
   `is_regression` decision logic has 4 unit tests covering all four
   Clean/Repaired/Degraded/Quarantined pairs; `std::fs::write` byte-restore
   is standard semantics.~~
   **Resolved post-adversarial review:** the post-validation logic is
   refactored into `finalise_post_writeback`, which routes both regression
   and validator-`Err` branches through `rollback_atomic` (NamedTempFile +
   fsync + `path_rename::commit`). Five new unit tests cover commit / rollback-on-regression /
   rollback-on-validator-error / atomic byte-restoration / no-orphan-tempfiles.

4. **Task 1's refactor IS a semantics change, not pure.** Per-entry
   compression preservation is now enforced (`repack::with_modifications`
   mirrors `file.compression()` instead of `FileOptions::default()`
   Deflate-everything). `repack_round_trip_preserves_per_entry_compression`
   is the regression test. Noted in the commit body.

5. **Path rename deferred from orchestrator happy path.** ~~`path_rename`
   module is complete with EXDEV fallback + collision check + tests, but
   the orchestrator's `run_once` keeps `src_path == dest_path` for MVP.~~
   **Resolved post-adversarial review:** `run_once` now calls
   `path_rename_step` (rendered via `path_template::render` against the
   primary author's `sort_name` + `works.title`), invokes
   `path_rename::move_existing` (same-FS rename + EXDEV fallback), and
   updates `manifestations.file_path`. Skipped when `library_path` is
   empty (test/dev guard). Covered by `render_target_path` unit tests +
   `run_once_renames_file_to_template_path` E2E DB test.

---

## Tasks Completed

| #   | Task                                                                    | Primary File                                         | Status |
| --- | ----------------------------------------------------------------------- | ---------------------------------------------------- | ------ |
| 1   | Extract `epub::repack::with_modifications` shared helper                | `backend/src/services/epub/repack.rs`                | ✅     |
| 2   | Migration: `writeback_jobs` + `file_hash` split + grants + RLS         | `backend/migrations/20260419000001_*.sql`            | ✅     |
| 3a  | Enrichment orchestrator: INSERT `writeback_jobs` in Apply arm           | `backend/src/services/enrichment/orchestrator.rs`    | ✅     |
| 3b  | Accept / revert routes: INSERT `writeback_jobs` in caller tx            | `backend/src/routes/metadata.rs`                     | ✅     |
| 4   | Writeback module root                                                   | `backend/src/services/writeback/mod.rs`              | ✅     |
| 5   | Worker queue with manifestation-aware NOT EXISTS CTE                    | `backend/src/services/writeback/queue.rs`            | ✅     |
| 6   | Per-job orchestrator (load → transform → repack → commit → hash)        | `backend/src/services/writeback/orchestrator.rs`     | ✅     |
| 7   | OPF rewrite (event-stream quick-xml transform)                          | `backend/src/services/writeback/opf_rewrite.rs`      | ✅     |
| 8   | Cover embed planning (replace + insert EPUB 2/3)                        | `backend/src/services/writeback/cover_embed.rs`      | ✅     |
| 9   | Path rename module with EXDEV fallback                                  | `backend/src/services/writeback/path_rename.rs`      | ✅     |
| 10  | Wire `spawn_worker` into `main.rs` on `reverie_app` pool                | `backend/src/main.rs`                                | ✅     |
| 11  | `WritebackConfig` + env vars + `.env.example`                           | `backend/src/config.rs` + `.env.example`             | ✅     |
| 12  | Webhook event stub (`emit_writeback_{complete,failed}`)                 | `backend/src/services/writeback/events.rs`           | ✅     |
| 13  | Repack round-trip tests (5 cases)                                       | `backend/src/services/epub/repack.rs`                | ✅     |
| 14  | Job emission integration tests (accept/revert/reject/double/enrichment) | `backend/src/routes/metadata.rs` + enrichment tests  | ✅     |
| 15  | OPF EPUB 2 vs 3 matrix + series + version preservation                  | `backend/src/services/writeback/opf_rewrite.rs`      | ✅     |
| 16  | OPF path discovery via `container.xml` (non-default path)               | `backend/src/services/writeback/orchestrator.rs`     | ✅     |
| 17  | Multiple `<dc:identifier>` + ISBN insertion                             | `backend/src/services/writeback/opf_rewrite.rs`      | ✅     |
| 18  | Custom `<meta>` + `<dc:coverage>` preservation                          | `backend/src/services/writeback/opf_rewrite.rs`      | ✅     |
| 19  | Cover embed tests (replace + insert EPUB 2/3)                           | `backend/src/services/writeback/cover_embed.rs`      | ✅     |
| 20  | Path rename matrix (collision + same-dir + EXDEV + normalise)           | `backend/src/services/writeback/path_rename.rs`      | ✅     |
| 21  | Post-validation rollback (atomic via tempfile + tests for regression AND validator-err) | `backend/src/services/writeback/orchestrator.rs`     | ✅     |
| 22  | Queue concurrency/retry/shutdown/max-attempts                           | `backend/src/services/writeback/queue.rs`            | ✅     |
| 23  | Crash-recovery reconciler                                               | `backend/src/services/writeback/queue.rs`            | ✅     |
| 24  | `current_file_hash` updates + `ingestion_file_hash` immutable           | `backend/src/services/writeback/orchestrator.rs`     | ✅     |

---

## Validation Results

| Check               | Result | Details                              |
| ------------------- | ------ | ------------------------------------ |
| `cargo fmt --check` | ✅     | No diffs                             |
| `cargo clippy -D warnings` | ✅     | Zero warnings                  |
| Unit tests          | ✅     | 231 passed, 0 failed                 |
| Integration tests   | ✅     | 55 passed, 0 failed (`--ignored`)    |
| Build               | ✅     | `cargo build` clean                  |
| Migration round-trip| ✅     | up → down → up all succeed           |
| `cargo audit`       | ⚠ pre-existing only | RSA RUSTSEC-2023-0071 is pre-existing; no new advisories from the writeback surface. |

---

## Files Changed

### Created

| File                                                                   | Lines |
| ---------------------------------------------------------------------- | ----- |
| `backend/migrations/20260419000001_add_writeback_pipeline.up.sql`       | +95   |
| `backend/migrations/20260419000001_add_writeback_pipeline.down.sql`     | +34   |
| `backend/src/services/epub/repack.rs`                                   | +291  |
| `backend/src/services/writeback/mod.rs`                                 | +23   |
| `backend/src/services/writeback/error.rs`                               | +29   |
| `backend/src/services/writeback/events.rs`                              | +40   |
| `backend/src/services/writeback/queue.rs`                               | +618  |
| `backend/src/services/writeback/orchestrator.rs`                        | +520  |
| `backend/src/services/writeback/opf_rewrite.rs`                         | +720  |
| `backend/src/services/writeback/cover_embed.rs`                         | +460  |
| `backend/src/services/writeback/path_rename.rs`                         | +180  |

### Updated

| File                                                       | Nature                                             |
| ---------------------------------------------------------- | -------------------------------------------------- |
| `backend/src/services/epub/mod.rs`                         | Register `pub mod repack;`                         |
| `backend/src/services/epub/repair.rs`                      | Refactor to call `repack::with_modifications`      |
| `backend/src/services/mod.rs`                              | Register `pub mod writeback;`                      |
| `backend/src/services/enrichment/orchestrator.rs`          | Emit `writeback_jobs` on Apply; integration test   |
| `backend/src/services/enrichment/queue.rs`                 | Rename `file_hash` → `ingestion_file_hash`         |
| `backend/src/services/enrichment/field_lock.rs`            | Rename `file_hash` → `ingestion_file_hash`         |
| `backend/src/services/ingestion/orchestrator.rs`           | Rename + set both hash columns at ingest          |
| `backend/src/services/metadata/draft.rs`                   | Rename in test fixture                             |
| `backend/src/models/work.rs`                               | Rename in test fixtures                            |
| `backend/src/test_support.rs`                              | Rename + add `WritebackConfig` literal             |
| `backend/src/routes/metadata.rs`                           | `enqueue_writeback` in `apply_version`/`clear_field` + tests |
| `backend/src/config.rs`                                    | `WritebackConfig` + env parsing + tests            |
| `backend/src/main.rs`                                      | Spawn `writeback::spawn_worker` on `reverie_app`   |
| `.env.example`                                             | 4 new `REVERIE_WRITEBACK_*` vars                   |

Total: 11 new, 14 updated, ~1700 LOC net added (excluding tests: ~1100 LOC).

---

## Tests Written

| Suite                                              | Cases |
| -------------------------------------------------- | ----- |
| `services::epub::repack`                           | 5 (unit)  |
| `services::writeback::opf_rewrite`                 | 8 (unit)  |
| `services::writeback::cover_embed`                 | 4 (unit)  |
| `services::writeback::path_rename`                 | 7 (unit)  |
| `services::writeback::orchestrator` (regression)   | 5 (unit: `is_regression` + `extract_opf_path`)    |
| `services::writeback::queue`                       | 6 (integration, `#[ignore]`)  |
| `services::writeback::orchestrator` (integration)  | 2 (integration, `#[ignore]`)  |
| `routes::metadata` (job emission)                  | 4 extensions + 1 new (`double_accept_enqueues_two_jobs`) |
| `services::enrichment::orchestrator` (job emission)| 1 extension in `autofill_applies_when_canonical_empty` |

---

## Next Steps

- [ ] Human review
- [ ] Open PR — description should include: "Depends on Step 7 (enrichment
      pipeline). Adds 4 new env vars (`REVERIE_WRITEBACK_{ENABLED,
      CONCURRENCY,POLL_IDLE_SECS,MAX_ATTEMPTS}`)."
- [ ] Ship as standalone commit; Step 11 (Library Health) can then consume
      `writeback_jobs` status + `current_file_hash != on_disk_hash`
      divergence.
- [ ] Step 12 (webhooks) upgrade: replace `services::writeback::events::*`
      stubs with real dispatcher calls when Step 12 lands — one-line edit
      per emitter.
