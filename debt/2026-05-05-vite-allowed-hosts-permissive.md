---
status: lifted
severity: low
surfaces: [developer, security]
adopted: 2026-05-05
adopted-because: cloud workspace hostnames unenumerable for static allowlist; OIDC dev flow needed proxy access; recognised as debt and recorded inline in vite.config.ts at adoption time
lift-when-class: internal-refactor
lift-when: UNK-168 (REVERIE_DEV_HOSTS env-driven allowlist) merged to main
lifted: 2026-05-07
superseded-by: PR <pending>
---

# Vite allowedHosts permissive in dev

## Constraint

Cloud dev environments (Coder, Codespaces, Gitpod) generate workspace
hostnames that are workspace-specific and unstable across rebuilds.
There is no way to enumerate them in advance for a static
`server.allowedHosts` list in `vite.config.ts`.

Vite's default DNS-rebinding guard checks the request `Host` header
against the allowlist; non-matching hosts get rejected. With the
allowlist set to a static localhost-only value, vite-served dev
bundles cannot be reached from the public-facing workspace URL —
breaking the entire active-dev iteration loop in cloud environments.

## Workaround

`frontend/vite.config.ts` sets two relaxations:

```ts
server: {
  host: true,            // bind on all interfaces
  allowedHosts: true,    // disable DNS-rebind guard entirely
  // ...
}
```

The inline comment (`vite.config.ts`) explicitly documents the
accepted risk:

> This widens the attack surface: the proxy block below forwards
> `/api`, `/auth`, and `/opds` to the backend, including
> authenticated routes ... With allowedHosts:true, a malicious page
> that successfully DNS-rebinds to the dev workstation can reach
> those backend routes through the dev proxy. The risk is accepted
> because (a) Vite is dev-only and never ships to production, ...

The comment also names the lift path:

> If you tighten this later, narrow allowedHosts to an env-driven
> allowlist (e.g. REVERIE_DEV_HOSTS) rather than restricting the
> proxy.

## Why this isn't the right shape

The accepted risk is real, even if narrow:

1. DNS-rebind attacks against developer workstations have been
   demonstrated against many dev tooling stacks (rails console, jenkins,
   Kubernetes dashboard). Reverie's `/auth/*` proxy means a
   successful attack reaches authenticated routes.
2. `allowedHosts: true` is a blunt instrument when a precise tool
   exists. The env-driven approach was named in the inline comment
   from day one — recording it as accepted debt avoids "we'll fix
   this someday" drift.
3. Once cloudflared ([UNK-164](https://linear.app/unkos/issue/UNK-164))
   stabilises the workspace hostname (`dev.reverie.unkos.net`), the
   allowlist becomes a small static set. The constraint that
   originally justified the workaround weakens substantially.

## Lift conditions

[UNK-168](https://linear.app/unkos/issue/UNK-168) — read
`REVERIE_DEV_HOSTS` env var at vite startup, parse comma-separated
hosts, pass as `server.allowedHosts`. Default to loopback-only when
unset. Land this **before** [UNK-164](https://linear.app/unkos/issue/UNK-164)
(cloudflared) so the tunnel's stable hostname can be allowlisted from
day one of cloudflared deployment.

## Lifted 2026-05-07

`frontend/vite-plugins/allowed-hosts.ts` parses
`REVERIE_DEV_HOSTS` (comma-separated) into the `server.allowedHosts`
list. When unset, the default is loopback-only (`localhost`,
`127.0.0.1`, `::1`). The inline security comment in
`vite.config.ts` was updated to reflect the new posture (DNS-rebind
guard active against a bounded allowlist). The env var is documented
in `frontend/CLAUDE.md` and `dev/README.md`. Cloud workspace
template (homelab repo) must export the variable matching the
workspace hostname; tracked separately.

## Related

- [UNK-168](https://linear.app/unkos/issue/UNK-168) — the env-driven
  allowlist ticket (lift trigger)
- [UNK-164](https://linear.app/unkos/issue/UNK-164) — cloudflared
  sidecar; consumes the allowlisted hostname
- `frontend/vite.config.ts` — workaround site (with inline
  documentation)
