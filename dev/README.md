# dev/

Local-development helper scripts and notes that are not part of the
runtime build.

## Scripts

### `seed-library.sh`

Populates `$REVERIE_LIBRARY_ROOT` with a curated set of public-domain
EPUBs from Standard Ebooks for backend integration tests and frontend
onboarding flows. No binaries are committed to the repo — the script
fetches at runtime.

## Environment variables

### `REVERIE_DEV_HOSTS`

Comma-separated list of **non-loopback** hostnames the Vite dev server
will accept in the request `Host` header. Bounds Vite's DNS-rebinding
guard.

Loopback hosts — `localhost`, any `*.localhost` subdomain, and any
IPv4 / IPv6 literal Host header — are accepted unconditionally by a
hardcoded short-circuit in Vite's host-validation middleware,
regardless of this allowlist. The allowlist only matters for
non-loopback hostnames a developer chooses to expose the dev bundle
under (cloud workspace URL, ngrok tunnel, reverse-proxy alias, etc).

```bash
# Local workstation — env var unset. Loopback access works via
# Vite's short-circuit; no other hostnames are accepted.
npm run dev

# Cloud dev environment (Coder, Codespaces) — set to the workspace's
# stable hostname so the dev bundle is reachable through it.
REVERIE_DEV_HOSTS=dev.reverie.unkos.net npm run dev

# Multiple hostnames — comma-separated.
REVERIE_DEV_HOSTS=dev.reverie.unkos.net,my-tunnel.ngrok.app npm run dev
```

Parsing lives in `frontend/vite-plugins/allowed-hosts.ts`; the inline
security comment in `frontend/vite.config.ts` documents the threat
model the allowlist closes.
