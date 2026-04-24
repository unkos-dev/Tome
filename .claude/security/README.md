# Reverie Security Reference

Canonical security rules for reverie-adjacent agent work. Imported verbatim from
an external source under CC BY 4.0 — consult the relevant file before completing
work that touches user input, authentication, authorization, sessions, secrets,
file I/O, XML parsing, outbound HTTP, or response headers.

These files complement (do not replace) the existing security tooling in this
repo: the `security-reviewer` agent, the `security-review` skill, the
`security-scan` skill, and the `prp-core:silent-failure-hunter` agent. Those
tools critique written code; these rules guide the initial draft.

## Source

- **Project:** [Project CodeGuard](https://github.com/cosai-oasis/project-codeguard) — Coalition for Secure AI (CoSAI), an OASIS Open Project
- **Upstream commit:** `00f6263a8fe992ad68222fd898551e5c7edbbaf9`
- **License:** [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) — attribution in each file's top-comment provenance header
- **Imported:** 2026-04-24

Reverie reproduces these files unmodified below the provenance header. No edits.
If a rule conflicts with reverie's stated position (see reverie's `CLAUDE.md`
Hard Rules), reverie's rule wins — document the deviation in this README's
"Deviations" section rather than editing the imported file.

## File manifest

### Core (foundational categories)

| File | Relevance to reverie |
|---|---|
| `codeguard-0-input-validation-injection.md` | SQL injection (sqlx), OS command injection, XML/LDAP injection |
| `codeguard-0-authentication-mfa.md` | OIDC via Authentik, device-token path, MFA posture |
| `codeguard-0-authorization-access-control.md` | Row-level security, IDOR prevention |
| `codeguard-0-session-management-and-cookies.md` | tower-sessions, cookie flags, expiry discipline |
| `codeguard-0-supply-chain-security.md` | cargo audit, npm audit, Docker image pinning, SBOM |
| `codeguard-0-file-handling-and-uploads.md` | EPUB ingestion path, magic-byte validation, safe storage |
| `codeguard-0-xml-and-serialization.md` | OPDS feed generation, EPUB XML parsing, XXE prevention |
| `codeguard-0-logging.md` | tracing discipline, redaction, no-secret-leakage |
| `codeguard-0-client-side-web-security.md` | Foundation for frontend hardening (see also UNK-106) |

### OWASP deep-dives (attack-specific)

| File | Relevance to reverie |
|---|---|
| `codeguard-0-content-security-policy.md` | UNK-106 direct input |
| `codeguard-0-http-headers.md` | UNK-106 direct input |
| `codeguard-0-http-strict-transport-security.md` | HSTS posture behind reverse proxy |
| `codeguard-0-clickjacking-defense.md` | Frame-ancestors / X-Frame-Options |

## How agents use this

When Hard Rule 6 (security scrutiny) fires on a change, open the file matching
the category you're touching. Apply the relevant rules. Answer "will this stand
up to a security review?" in the task summary with a reference to the specific
rule sections consulted.

For categories not covered here (e.g., mobile, k8s, C-language safety), use the
external canon pathway: WebFetch from the upstream repo at the pinned commit.

## Refresh policy

Re-pull from upstream when:

- A Reverie security review surfaces a gap that's since been covered upstream
- Upstream publishes a significant version update (watch releases)
- At minimum: **quarterly check** against upstream `main`

Refresh process:

1. Note the new upstream commit SHA
2. Run the import script (see commit message of the initial import for the shell
   command that fetched these files) with the new SHA
3. Review the diff against your local files — flag any content change that
   conflicts with reverie's existing rules
4. Update this README's "Upstream commit" field and "Imported" date
5. Commit as `chore(security): refresh codeguard rules (<short-sha-old>..<short-sha-new>)`

## Deviations

_(None currently. Log any reverie-specific overrides here, with rationale and
the specific file + section being overridden.)_
