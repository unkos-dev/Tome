---
title: Introduction
description: What is Reverie and how to get started.
---

Reverie is a self-hosted ebook library manager built with Rust and React.

## Quick Start

> **Pre-alpha note:** No semver release has been cut yet, so the
> conventional `latest` tag on `ghcr.io/unkos-dev/reverie` is
> intentionally unset. Until the first `v0.1.0` ships, only the floating
> `main` tag exists, and it is **`linux/arm64` only** — amd64 users must
> wait for the first semver release. Track
> [Releases](https://github.com/unkos-dev/reverie/releases) for the first
> `vX.Y.Z` tag; once it ships, replace `:main` below with `:vX.Y.Z` (a
> multi-arch manifest).

```bash
docker pull ghcr.io/unkos-dev/reverie:main
```

```bash
docker run -p 3000:3000 ghcr.io/unkos-dev/reverie:main
```

> **Note:** Reverie is in pre-alpha. These instructions will be expanded as the project matures.
