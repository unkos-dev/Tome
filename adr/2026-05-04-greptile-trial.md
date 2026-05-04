---
status: proposed
date: 2026-05-04
decision-makers: john
---

# Greptile AI code review: 4-week trial

## Context and Problem Statement

Reverie has one human reviewer (the maintainer). Reviewer attention
is the bottleneck — every PR sits until the maintainer can look at
it. The strict-lint policy ratified in
[`adr/2026-05-03-strict-lint-policy.md`](2026-05-03-strict-lint-policy.md)
machine-enforces style and most CLAUDE.md hard rules, but the
reviewer remains the only check on logic-level concerns: subtle data
flow bugs, missed edge cases, security gaps not caught by lint, and
deviations from project conventions that lint can't express.

Greptile is a third-party AI code reviewer (GitHub App) that
auto-comments on PRs with a graph-based view of the codebase. Two
questions to answer in this trial:

1. **Signal**: does Greptile catch enough real issues (logic bugs,
   security gaps, convention violations) to justify the noise?
2. **Workflow fit**: can a single-maintainer project absorb auto
   comments without them becoming review fatigue?

The strict-lint policy intentionally landed first to compress the
style territory Greptile would otherwise claim, isolating the trial
to logic-level signal where AI review is most likely to add value.

## Decision

Run a 4-week trial of Greptile on `unkos-dev/reverie`.

### Trial configuration

Maximally verbose for trial calibration. Better to start with full
visibility into what Greptile catches and tighten later than to
start tight and miss patterns.

`greptile.json` at repo root:

- `strictness: 1` — counter-intuitively, this is the *least*
  filtered setting in Greptile's schema (1 = Low strictness =
  Verbose; 3 = High strictness = Critical-only). Comments on
  everything Greptile flags. Trial purpose is signal calibration
  — needs the full output to evaluate
- `commentTypes: ["logic", "syntax", "style", "info"]` — every
  category enabled. Style overlap with the lint policy is expected
  signal data: any style nit Greptile raises that lint already
  enforces is a data point on Greptile's lint awareness; any nit
  lint doesn't catch is a candidate for a new lint rule
- `triggerOnUpdates: true` — re-review on every push, not just first
- `customContext.files`: pinned references to `CLAUDE.md`,
  `backend/CLAUDE.md`, `frontend/CLAUDE.md`,
  `adr/2026-05-03-strict-lint-policy.md`, and the 13
  `.claude/security/codeguard-*.md` checklists. These are the
  load-bearing convention sources — Greptile should read them
  before flagging style/security
- `customContext.rules`: project-specific rules with file scopes —
  `time` not `chrono` (backend), no raw hex literals (frontend), no
  `enum` (frontend), no inline JSX style (frontend), shadcn/ui
  carve-out, secret-handling stance, TDD requirement, Conventional
  Commits requirement
- No `ignorePatterns` initially — see Alternatives. Amended
  2026-05-04 to exclude lockfiles only; see Amendments below

### App install

Done by the maintainer via GitHub App marketplace
(<https://github.com/apps/greptileai>) on the `unkos-dev/reverie`
repo only (not org-wide).

### Trigger model

Auto on every PR. No label gate. Branch filtering (skip
`release-please--*` and `dependabot/*`) considered but not adopted
in this trial — those PRs are typically small and Greptile silence
on them is itself a useful data point.

### Trial gate

**Duration**: 4 weeks from install date.

**Success metric**: ≥30% of Greptile findings actionable (defined
as: surfaces a real issue and gets addressed in a follow-up commit
on the same PR, or filed as a tracked issue). Findings dismissed
without action count against. Maintainer logs verdict per PR in a
running tally during the trial.

**At the gate**, decide:

- **Adopt**: keep current config, possibly tighten `commentTypes`
  to drop `style` and `info` if they're the noise drivers
- **Reconfigure and re-trial**: change `strictness`, `commentTypes`,
  or add `ignorePatterns`; another 4 weeks
- **Reject**: uninstall App, delete `greptile.json`. No repo
  changes (lint policy stands on its own)

Decision recorded as a follow-up ADR (`accepted` / `superseded`).

## Consequences

* Good — second reviewer on every PR. Single-maintainer projects
  rely on lint + manual review; an AI reviewer adds a third pass
  that catches what either misses
* Good — graph-based context (the Greptile differentiator) means
  comments can reference cross-file patterns, not just the diff
  in isolation. Stronger than per-file static analysis
* Good — pinned `customContext.files` align Greptile with project
  conventions from day one. Reduces the cold-start noise common in
  AI reviewers that don't read the codebase's documented rules
* Good — `strictness: 1` + all `commentTypes` produces maximum
  signal data for the trial verdict. Easier to dial up (raise the
  filter to 2 or 3) at the gate with evidence than to dial down
  from a quiet starting point and miss the patterns Greptile only
  flags at the verbose tier
* Bad — auto-review on every PR means review noise during the
  trial. Maintainer must triage every comment, even false
  positives, to populate the actionable-rate metric
* Bad — third-party data exposure. Greptile reads PR diffs and
  full repo context. Reverie is open-source so the surface is
  public, but the org-internal `.claude/`, `adr/`, and
  `docs/superpowers/specs/` paths are also read. Acceptable for
  this repo; not transferable to private repos without re-review
* Bad — third-party SaaS dependency. If Greptile changes pricing,
  policy, or API, the workflow regresses. Mitigated by trial
  framing — no automation depends on Greptile's existence
* Neutral — `customContext` may inflate per-PR token spend on
  Greptile's side. Visible in their billing dashboard (if relevant
  for the trial tier) but not on our infra
* Neutral — Greptile's findings are advisory. Maintainer remains
  the sole approver for merge

## Alternatives Considered

* **No AI reviewer (status quo).** Rejected — the open question
  ("does AI review add value here?") doesn't get answered without
  a trial. Lint + manual review is the baseline; the trial measures
  the marginal value of an AI layer on top
* **CodeRabbit / Diamond / Codium / other AI reviewers.** Deferred
  — Greptile's graph-based codebase context is the most
  differentiated angle, and 4 weeks of one tool generates cleaner
  signal than 1 week of four. If the Greptile trial fails, the next
  ADR can pick up an alternative with a clean comparison baseline
* **Self-hosted AI reviewer (Claude Code in CI, custom bot).**
  Rejected for now — trial framing values "does AI review work for
  this repo at all" over "which implementation is best." A
  managed service with one config file is the lowest-effort
  hypothesis test
* **`strictness: 2` (balanced default) or `strictness: 3`
  (critical-only) for trial.** Rejected — both pre-filter the
  output before the trial has measured what the unfiltered output
  looks like. `strictness: 1` exposes the full noise floor; the
  trial verdict can then justify raising the filter (to 2 or 3) at
  the gate with evidence. Starting filtered and trying to
  reconstruct what was hidden is harder than starting verbose and
  pruning
* **`commentTypes: ["logic"]` only.** Rejected — pre-filters out
  the data needed to evaluate Greptile's full output. Trial first,
  filter later
* **Add `ignorePatterns` for `package-lock.json`, `Cargo.lock`,
  `dist/`, `graphify-out/`, generated SQL.** Considered. Skipped
  in initial config because no per-PR cost data exists yet to
  justify the speedup. Add at the trial gate if review latency or
  cost is the binding constraint. **Amended 2026-05-04**: the
  lockfile subset (`package-lock.json`, `Cargo.lock`) was added
  early in response to a different binding constraint than the one
  this bullet anticipated — see Amendments below. The remaining
  patterns (`dist/`, `graphify-out/`, generated SQL) stay deferred
* **Branch filter to skip `release-please--*` and `dependabot/*`.**
  Rejected for the trial — silence on these branches is itself
  useful signal. If Greptile adds noise on dependency bumps, that's
  a config issue worth catching during the trial, not pre-empting
* **Opt-in via `greptile-review` label instead of auto-on-every-PR.**
  Rejected — opt-in produces selection bias (only complex PRs get
  reviewed), undermining the actionable-rate metric. Auto-on-all
  exposes Greptile to the full PR mix and gives the trial verdict
  honest data
* **Org-wide install instead of per-repo.** Rejected — Reverie is
  the only repo in scope. Per-repo install matches the trial
  scope; org-wide install pre-commits to other repos before the
  verdict is in

## Amendments

### 2026-05-04 — `ignorePatterns` added for lockfiles (PR #148)

The initial config in this ADR deferred `ignorePatterns` to the
trial gate, with the explicit trigger: *"Add at the trial gate if
review latency or cost is the binding constraint."* That trigger
described a future quantitative ceiling — too slow or too expensive
once data accumulated.

A different binding constraint emerged ~5 hours into the trial,
not anticipated by the original ADR: a **confirmed hallucination
pattern** specific to lockfiles. Two consecutive Renovate npm-bump
PRs (#71 `@commitlint/config-conventional` and #74
`markdownlint-cli2`) produced identical false-positive findings —
Greptile narrated the existing `name: reverie-dev` string in
`package-lock.json` context as a brand-new `tome-dev → reverie-dev`
"silent rename" introduced by the PR. Verified against actual
diffs: zero `name` field changes in either PR. The pattern is
consistent (twice in two consecutive lockfile PRs), specific
(lockfile only), and predictable (will repeat on every Renovate
npm bump touching `package-lock.json` until mitigated).

PR #148 adds `**/package-lock.json` and `**/Cargo.lock` to
`ignorePatterns` to suppress this false-positive class. Lockfile
changes are mechanical regenerations of dep trees plus integrity
hashes; line-by-line review has zero security signal beyond what
npm/cargo already verify cryptographically. The mitigation also
saves Greptile review-credit budget on every Renovate PR (50/account
trial cap, already partially burned by the eslint v10 retry storm
documented separately in PR #147).

The remaining deferred patterns (`dist/`, `graphify-out/`,
generated SQL) stay deferred per the original ADR — they have not
exhibited the false-positive class that justified the lockfile
amendment.

Trial tally tracking these findings: UNK-155 (rows #4 and #5).

## More Information

* MADR 4.0: <https://adr.github.io/madr/>
* Greptile docs: <https://docs.greptile.com>
* `greptile.json` schema: <https://docs.greptile.com/code-review/configuration>
* Related: [`adr/2026-05-03-strict-lint-policy.md`](2026-05-03-strict-lint-policy.md) — strict lint
  landed first to compress style territory before this trial
* Related: `CLAUDE.md` "Hard Rules" §5 (TDD), §6 (security
  scrutiny), §7 (secret handling) — Greptile is configured to
  read these via `customContext.files`
