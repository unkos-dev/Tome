# Image Build Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire GHA build cache + cargo-chef 4-stage Dockerfile onto `docker-publish.yml` so backend deps survive across runs, per-arch.

**Architecture:** Refactor `Dockerfile` backend stage into cargo-chef chef/planner/cooker/backend-builder split so dep compilation lands in a cacheable layer. Wire `docker/build-push-action@v7` with `type=gha` cache scoped per matrix arch. Add Tier 1 observability (`buildx du` + step summary) post-build for visibility.

**Tech Stack:** Docker buildkit, cargo-chef 0.1.77, `docker/build-push-action@v7`, `type=gha` cache backend, GitHub Actions `$GITHUB_STEP_SUMMARY`.

**Spec:** `plans/2026-05-12-image-build-cache-design.md`

**Linear ticket:** [UNK-246](https://linear.app/unkos/issue/UNK-246/featci-gha-build-cache-cargo-chef-dockerfile)

**Branch:** `feat/unk-246-image-build-cache`

---

## File Structure

**Modify:**

- `Dockerfile` — replace 2-stage backend+frontend with 4-stage backend (chef/planner/cooker/backend-builder) + frontend with cache mount; runtime unchanged.
- `.github/workflows/docker-publish.yml` — add `cache-from`/`cache-to` to `build-push-action` step; append `buildx du` step + `$GITHUB_STEP_SUMMARY` step post-build.

**No new files.** No automated test harness (per spec Section 4 — verification is "green build + CACHED line in warm run").

**No tests modified.** Build/CI config; behavior verified via push-and-observe on feature branch.

---

## Task 1: Refactor Dockerfile to cargo-chef 4-stage backend

**Files:**

- Modify: `Dockerfile` (full backend section rewrite + frontend cache mount addition)

- [ ] **Step 1: Replace Dockerfile contents**

Open `Dockerfile`. Replace entire current contents with:

```dockerfile
# syntax=docker/dockerfile:1.7

# Stage 1a: chef base — pinned cargo-chef install shared across planner + cooker.
# Version pin prevents recipe.json schema drift between planner emit and cooker
# consume. Bump in lockstep across both stages (they inherit from this base).
FROM rust:1-slim AS chef
RUN cargo install cargo-chef@0.1.77 --locked
WORKDIR /build

# Stage 1b: planner — emits recipe.json describing the dependency tree.
# Cheap stage (no compilation); recipe.json hash drives cooker cache key.
FROM chef AS planner
COPY backend/ .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 1c: cooker — compiles deps only, from recipe.json.
# This layer is the cache target — warm hits skip ~3min of dep compilation.
FROM chef AS cooker
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 1d: backend-builder — real build atop warm dep layer.
# SQLX_OFFLINE forces sqlx::query! macros to validate against the committed
# .sqlx/ cache instead of opening a database connection at compile time.
# Cache regeneration: `cargo sqlx prepare -- --tests` against a populated dev DB.
FROM cooker AS backend-builder
COPY backend/ .
ENV SQLX_OFFLINE=true
RUN cargo build --release

# Stage 2: Build frontend with buildkit npm cache mount.
# /root/.npm survives across runs scoped to the builder, independent of layer
# cache; mount is buildkit-scoped not layer-scoped so it survives base-image swaps.
FROM node:24.15.0-slim AS frontend-builder
WORKDIR /build
COPY frontend/package.json frontend/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci
COPY frontend/ .
RUN npm run build

# Stage 3: Runtime
FROM debian:bookworm-slim AS runtime
# UNK-165: curl is the HTTP client used by the HEALTHCHECK below; readiness
# probe needs a working HTTP client baked in so docker / compose / Incus can
# detect a successful migration window before flipping traffic.
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
RUN useradd -r -s /bin/false reverie

COPY --from=backend-builder /build/target/release/reverie-api /usr/local/bin/reverie-api
COPY --from=frontend-builder /build/dist /srv/frontend
# UNK-106: the backend serves /assets/* and falls back to index.html for SPA
# routes when this env var is set. Validation at startup panics the process
# if the dir or its csp-hashes.json sidecar is missing.
ENV REVERIE_FRONTEND_DIST_PATH=/srv/frontend

USER reverie
EXPOSE 3000

# UNK-165: probe the readiness endpoint (DB-dependent) so the container is
# only reported healthy once migrations are applied and the pool is live.
# 60s start-period covers the migration window for first boot.
HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=3 \
    CMD curl --fail --silent --show-error --output /dev/null http://127.0.0.1:3000/health/ready

ENTRYPOINT ["reverie-api"]
```

> **Deviation 2026-05-13:** Local Docker sanity steps (cold build, warm build, healthcheck inspection) dropped. Spec acceptance is "green CI build"; local builds duplicate CI verification at ~5-10min wall-clock cost per leg on this workspace. Verification now lives entirely in Tasks 4-5 (push branch → observe CI).

- [ ] **Step 2: Commit**

```bash
git add Dockerfile
git commit -m "refactor(docker): split backend into cargo-chef stages

Introduces planner/cooker/backend-builder split so dependency
compilation lands in a dedicated layer keyed on recipe.json. Cache
target for the GHA build-cache wiring landing in the same PR.

Frontend stage gains a buildkit npm cache mount on /root/.npm so
package downloads survive across runs independent of layer cache.

Runtime stage unchanged.

Refs: plans/2026-05-12-image-build-cache-design.md"
```

---

## Task 2: Wire GHA cache on docker-publish.yml

**Files:**

- Modify: `.github/workflows/docker-publish.yml` (build job — `docker/build-push-action@v7` step inputs)

- [ ] **Step 1: Add cache-from + cache-to inputs**

Open `.github/workflows/docker-publish.yml`. Locate the `Build and push by digest` step (currently lines 95–104). Replace the `with:` block:

```yaml
- name: Build and push by digest
  id: build
  uses: docker/build-push-action@v7
  with:
    context: .
    platforms: ${{ matrix.platform }}
    labels: ${{ steps.meta.outputs.labels }}
    outputs: type=image,name=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }},push-by-digest=true,name-canonical=true,push=true
    provenance: mode=max
    sbom: true
    cache-from: type=gha,scope=buildcache-${{ matrix.arch }}
    cache-to: type=gha,scope=buildcache-${{ matrix.arch }},mode=max
```

Only the last two lines are new. Order matters only for readability.

- [ ] **Step 2: actionlint locally**

```bash
actionlint .github/workflows/docker-publish.yml
```

Expected: no output (clean).

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/docker-publish.yml
git commit -m "feat(ci): wire GHA build cache per-arch on docker-publish

type=gha cache backend with scope=buildcache-\${matrix.arch} keeps
amd64 and arm64 caches isolated. mode=max exports intermediate
layers so partial-hit scenarios still benefit.

Cache miss = cold build (no correctness risk). Cache backend
unreachable = silent fallthrough. Rollback = revert.

Refs: plans/2026-05-12-image-build-cache-design.md"
```

---

## Task 3: Add Tier 1 observability steps

**Files:**

- Modify: `.github/workflows/docker-publish.yml` (build job — append two steps after `Build and push by digest`)

- [ ] **Step 1: Insert observability steps**

Open `.github/workflows/docker-publish.yml`. Locate the end of the `Build and push by digest` step (the step whose `id: build` was modified in Task 2). Insert these two steps immediately after it, before the `Export digest` step:

```yaml
# Tier 1 observability — surfaces cache utilisation per run without
# external infra. `if: always()` so failures don't suppress diagnostic
# output. Adopt Tier 2 (weekly cache-inventory cron) only on revisit
# conditions per ADR.
- name: Cache disk usage
  if: always()
  run: docker buildx du

- name: Step summary
  if: always()
  env:
    ARCH: ${{ matrix.arch }}
  run: |
    {
      echo "## Build cache report (arch=${ARCH})"
      echo
      echo "Cache disk usage above. Inspect 'Build and push by digest'"
      echo "step log for per-stage \`CACHED\` lines to confirm cooker"
      echo "layer reuse."
    } >> "$GITHUB_STEP_SUMMARY"
```

Note: step-summary parsing of `CACHED` lines from buildx log is deferred. Spec flagged this as open item. Minimal version above directs human reader to the build log without log-scraping; if log-scraping becomes valuable, add follow-up plan.

- [ ] **Step 2: actionlint locally**

```bash
actionlint .github/workflows/docker-publish.yml
```

Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/docker-publish.yml
git commit -m "feat(ci): tier 1 build-cache observability

Adds two post-build diagnostic steps:
- docker buildx du dumps cache disk usage to step log
- \$GITHUB_STEP_SUMMARY pointer to per-stage CACHED lines

Zero infra. No alerting. Tier 2/3 (cache-inventory cron, OTLP,
sccache stats) deferred to revisit conditions.

Refs: plans/2026-05-12-image-build-cache-design.md"
```

---

## Task 4: Push branch — verify cold build green

**Files:** none (verification only)

- [ ] **Step 1: Push branch**

```bash
git push -u origin feat/unk-246-image-build-cache
```

- [ ] **Step 2: Watch workflow run**

```bash
gh run watch --workflow=docker-publish.yml
```

Or via UI:

```bash
gh run list --workflow=docker-publish.yml --limit 1
```

Expected: cold build runs end-to-end. arm64 leg only (main-push channel — not a tag-push). Build duration likely close to current baseline (4m35s) since first run is cold.

- [ ] **Step 3: Confirm step summary present**

```bash
gh run view --log $(gh run list --workflow=docker-publish.yml --limit 1 --json databaseId -q '.[0].databaseId') | grep -E "(buildx du|Build cache report)" | head -5
```

Expected: lines from both observability steps appear.

- [ ] **Step 4: Confirm cache populated**

```bash
gh api /repos/unkos-dev/reverie/actions/caches --jq '.actions_caches[] | select(.key | startswith("buildcache-arm64"))'
```

Expected: at least one entry under `buildcache-arm64` scope. Size > 0.

If no cache entries appear: `cache-to` wiring failed silently. Inspect build step log for cache-export errors before proceeding.

---

## Task 5: Push trivial change — verify warm build CACHED

**Files:**

- Modify: `backend/src/main.rs` (whitespace-only edit — to be reverted in Task 6)

- [ ] **Step 1: Trivial src edit**

Append a blank line to `backend/src/main.rs`:

```bash
echo "" >> backend/src/main.rs
```

- [ ] **Step 2: Commit + push**

```bash
git add backend/src/main.rs
git commit -m "test(ci): trivial src edit to verify warm-cache reuse

Temporary commit — reverted in next commit. Validates that the
cooker stage hits the GHA cache when only app-crate source changes
(no Cargo.toml/lock churn).

Refs: plans/2026-05-12-image-build-cache-design.md"
git push
```

- [ ] **Step 3: Watch workflow + inspect build log**

```bash
gh run watch --workflow=docker-publish.yml
RUN_ID=$(gh run list --workflow=docker-publish.yml --limit 1 --json databaseId -q '.[0].databaseId')
gh run view --log "$RUN_ID" | grep -E "CACHED|importing cache manifest|exporting cache" | head -20
```

Expected: log shows `CACHED [cooker N/M]` line for the cooker stage. `importing cache manifest from gha` line confirms cache-from hit. `backend-builder` stage recompiles (small + fast).

If cooker does NOT show CACHED: cache-from is not hitting. Likely causes (in order): scope mismatch, recipe.json hash drift, GHA cache backend unreachable. Inspect step log before proceeding.

---

## Task 6: Revert trivial commit + open PR

**Files:**

- Modify: `backend/src/main.rs` (revert Step 1 of Task 5)

- [ ] **Step 1: Revert trivial edit**

```bash
git revert --no-edit HEAD
```

This produces a new commit that reverses the whitespace addition. Do not amend or rebase — preserve audit trail of the verification step.

- [ ] **Step 2: Push revert**

```bash
git push
```

- [ ] **Step 3: Watch final run on branch**

```bash
gh run watch --workflow=docker-publish.yml
```

Expected: another warm-cache run. cooker still CACHED. Branch state now matches main except for Dockerfile + workflow changes.

- [ ] **Step 4: Open PR**

```bash
gh pr create --title "feat(ci): GHA build cache + cargo-chef Dockerfile (UNK-246)" --body "$(cat <<'EOF'
## Summary

- Refactors Dockerfile backend stage into cargo-chef 4-stage split (chef/planner/cooker/backend-builder) so dep compilation lands in a cacheable layer.
- Wires `type=gha` cache on `docker/build-push-action@v7` with per-arch scope (`buildcache-amd64`, `buildcache-arm64`).
- Adds Tier 1 observability post-build: `docker buildx du` + `$GITHUB_STEP_SUMMARY` pointer.
- Frontend stage gains buildkit npm cache mount on `/root/.npm`.

## Why

Future-proofing for backend dep growth (currently 70 direct, will climb). Cold rebuild cost compounds across two arches at tag-push. GHA cache is free, zero infra, and the 10GB pool is currently 0% utilised.

Spec: `plans/2026-05-12-image-build-cache-design.md`.

## Verification

- Cold build green on first push (cache miss as expected).
- Warm build (trivial src edit) shows `CACHED [cooker]` line — confirms cache loop closed.
- `gh api /repos/unkos-dev/reverie/actions/caches` shows populated `buildcache-arm64` scope.

No automated test harness — verification is empirical via push-and-observe per spec Section 4.

## Rollback

Single-commit revert. No infra to unwind, no data migration, no external state.

## Test plan

- [ ] Cold build on feature branch is green
- [ ] Warm build shows `CACHED [cooker N/M]` line
- [ ] `gh api .../actions/caches` lists `buildcache-arm64` entry > 0 bytes
- [ ] Step summary renders with cache report markdown
- [ ] actionlint clean locally + in CI
EOF
)"
```

- [ ] **Step 5: Confirm PR is green + hand off to user**

```bash
gh pr checks
```

Wait for all checks to pass. Once green, stop. User reviews and merges. Do not merge.

---

## Self-Review Notes

**Spec coverage:**

- Section 1 (Architecture) → Task 1 (Dockerfile) + Task 2 (workflow cache wiring).
- Section 2 (Lifecycle) → verified by Task 4 (cold) + Task 5 (warm).
- Section 3 (Failure modes + Tier 1 obs) → Task 3 (obs steps); cache-miss-is-safe behaviour confirmed by Task 4 first-push.
- Section 4 (Verification) → Task 4 + Task 5 explicit.
- Section 5 (Revisit conditions) → documented in spec + ADR; no implementation task needed.
- Open items (cargo-chef pin, cache syntax, summary parser, combined PR) → resolved: 0.1.77, syntax confirmed against build-push-action v7 docs, summary parser deferred as documented in Task 3 note, combined PR per default.

**Type/name consistency:**

- Stage names (`chef`, `planner`, `cooker`, `backend-builder`, `frontend-builder`, `runtime`) consistent across Task 1 + spec.
- Scope name (`buildcache-${{ matrix.arch }}`) consistent across Tasks 2/4.
- Workflow step IDs preserved (`build`, `meta`).

**Placeholders:**

- `UNK-???` ticket ID — flagged as branch-rename pre-req; Linear filing is downstream of plan review.
- No TBD/TODO in step bodies. cargo-chef pin is concrete (0.1.77).

---

## Downstream (post-merge)

- Write ADR via `adr` skill: `adr/YYYY-MM-DD-image-build-cache.md`. Status `accepted` once PR merges. References spec + this plan. **Carry-over points enumerated below.**
- Close Linear ticket UNK-246 with PR link.
- Monitor next ~5 main-push runs for cache hit consistency. If cooker repeatedly cold-rebuilds without Cargo.lock churn: hit revisit conditions.

---

## ADR carry-over

Points surfaced during planning + review that need to land in the ADR so they're not lost. Sourced from spec, brainstorming session, bot review triage on PR #233, and adversarial review post-implementation.

### Decision rationale to record

1. **cargo-chef over plain buildkit cache mounts** — alternative considered: `RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/build/target cargo build --release`. Rejected: cache mounts give cross-run cargo registry + target reuse but no dedicated layer for deps vs app code, so app-only edits still re-link. cargo-chef's planner/cooker split puts dep compilation in its own gha-exported layer. Cost: one pinned third-party crate + ~30-60s `cargo install` on cold chef-layer rebuilds.

2. **Per-arch cache scope (`buildcache-${{ matrix.arch }}`)** — amd64 and arm64 builds keep independent cache namespaces. mode=max exports intermediate layers so partial-hit scenarios still benefit.

3. **Scope key is branch-agnostic on purpose** — GHA Actions cache partitions entries by `GITHUB_REF` with read-fallback from base ref. Embedding `${{ github.ref_name }}` in the scope would _defeat_ that fallback. Same scope name across branches IS the correct shape; GHA-level partitioning handles cross-branch isolation. (Source: CodeRabbit review thread on PR #233, dismissed with this rationale.)

4. **Tier 1 observability boundary** — `docker buildx du` + `$GITHUB_STEP_SUMMARY` pointer. No log-scraping, no external telemetry, no cron. Tier 2/3 (cache-inventory cron, OTLP, sccache stats) explicitly deferred to revisit conditions; cost not yet earned.

5. **`workflow_dispatch` trigger added during implementation** — discovered Task 4 verification path was wrong (workflow only fired on `main` + `v*`); manual trigger enables ad-hoc rebuilds + feature-branch CI verification of workflow changes that wouldn't otherwise fire. Permanent addition.

### Threats / failure modes to record

1. **Cache miss is never a correctness risk** — `cache-from` miss = cold build; `cache-to` unreachable = silent fallthrough. No data corruption surface.

2. **Floating base-image tags (`rust:1-slim`, `node:24.15.0-slim`, `debian:bookworm-slim`)** — patch/minor updates cascade-invalidate the `chef` layer (~30-60s `cargo install cargo-chef` re-run + cooker/backend-builder re-compile). Pre-existing pattern in this repo. Note in revisit conditions; digest-pinning is a project-wide call deferred to a follow-up.

3. **Tag-push perf is unverified** — verification ran on feature-branch → feature-branch round-trip (cold 6m32s → warm 2m38s). First tag-push after merge reads from GHA cache with `base = main` fallback; behavior for tag refs is documented-ambiguous and empirical. Likely scenarios:
   - Tag base-ref fallback hits main's cache → warm tag-push.
   - Base-ref fallback misses for tag refs → cold tag-push on both arches (~5-7min × 2 native runners).
   - Either is functionally correct; perf-only.
   - Action: record actual tag-push wall-clock on first `v*` after merge. Add a `debt/` entry if tag-push consistently runs cold.

4. **Plan deviations during implementation** — three items not in original plan:
   - Local Docker sanity checks dropped (recorded as deviation in plan Task 1).
   - `workflow_dispatch` trigger added (this carry-over section is the record).
   - Frontend cache-mount comment + Tier 1 obs language rewritten after Greptile/CR review (commit `a46878f`).

### Rollback path

1. **Single-commit revert.** No infra to unwind, no data migration, no external state. Acceptance is "build still green"; no perf threshold the rollback needs to clear.

### Revisit conditions (re-stated from spec for ADR convenience)

Flip cache shape when:

- Cooker not `CACHED` across consecutive src-only pushes → investigate 10GB cap eviction → consider `type=registry,ref=ghcr.io/unkos-dev/reverie:buildcache` for unbounded retention.
- Backend dep count crosses ~150 OR cooker cold-rebuild crosses ~8min → consider sccache layer atop cargo-chef.
- Per-PR builds added → re-evaluate (cache pollution risk).
- buildkit/GHA cache backend deprecated or pricing changes → forced revisit; registry cache is obvious fallback.
- External contributors arrive → CI cost visibility matters more; Tier 2 cache-inventory cron worth the effort.
- Tag-push runs consistently cold (see #8) → consider explicit cross-ref cache hydration step on tag-push.
