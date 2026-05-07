// HMR client-port resolver. The browser's HMR websocket reconnect URL
// uses the dev server's port by default; when the dev server is fronted
// by a reverse proxy on a different port (e.g. cloudflared exposing
// `:5173` at `:443` via dev.reverie.unkos.net), the client must be told
// the externally-visible port or it tries `wss://<host>:5173/` and
// fails. REVERIE_DEV_HMR_CLIENT_PORT carries that override; unset =
// localhost-style same-port reconnect.

export interface HmrConfig {
  hmr: { clientPort: number };
}

export function parseHmrConfig(
  envValue: string | undefined,
): HmrConfig | Record<string, never> {
  if (envValue === undefined || envValue.trim().length === 0) return {};
  const port = Number(envValue);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(
      `REVERIE_DEV_HMR_CLIENT_PORT must be an integer in 1..=65535, got ${JSON.stringify(envValue)}`,
    );
  }
  return { hmr: { clientPort: port } };
}
