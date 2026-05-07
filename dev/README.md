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

Comma-separated list of hostnames the Vite dev server will accept in
the request `Host` header. Bounds Vite's DNS-rebinding guard.

```bash
# Local workstation — env var unset; defaults to loopback (localhost,
# 127.0.0.1, ::1). Nothing else can reach the dev bundle.
npm run dev

# Cloud dev environment (Coder, Codespaces) — set to the workspace's
# stable hostname. Replaces the loopback defaults.
REVERIE_DEV_HOSTS=dev.reverie.unkos.net npm run dev

# Multiple hostnames — comma-separated.
REVERIE_DEV_HOSTS=dev.reverie.unkos.net,localhost npm run dev
```

The value is a strict replacement of the default loopback list. If a
cloud workspace also wants loopback access, include `localhost` (and
optionally `127.0.0.1`, `::1`) in the comma-separated list.

Parsing lives in `frontend/vite-plugins/allowed-hosts.ts`; the
inline security comment in `frontend/vite.config.ts` documents the
threat model the allowlist closes.
