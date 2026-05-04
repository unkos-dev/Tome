# Architecture Decision Records (ADR)

An Architecture Decision Record (ADR) captures an important architecture decision along with its context and consequences.

## Conventions

- Directory: `adr`
- Naming:
  - Use date-prefixed files: `YYYY-MM-DD-choose-database.md`
  - If the repo already uses slug-only names, keep that: `choose-database.md`
- Status values: `proposed`, `accepted`, `rejected`, `deprecated`, `superseded`

## Workflow

- Create a new ADR as `proposed`.
- Discuss and iterate.
- When the team commits: mark it `accepted` (or `rejected`).
- If replaced later: create a new ADR and mark the old one `superseded` with a link.

## ADRs

- [Adopt architecture decision records](2026-04-30-adopt-architecture-decision-records.md) (accepted, 2026-04-30)
- [Strict lint policy: clippy pedantic + ESLint strict-tier](2026-05-03-strict-lint-policy.md) (proposed, 2026-05-03)
- [Greptile AI code review: 4-week trial](2026-05-04-greptile-trial.md) (proposed, 2026-05-04)
- [Replace eslint-plugin-react with @eslint-react/eslint-plugin](2026-05-04-replace-eslint-plugin-react.md) (accepted, 2026-05-04)