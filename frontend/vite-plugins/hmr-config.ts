// When the dev server is fronted by a reverse proxy on a different
// external port (e.g. a Cloudflare tunnel terminating TLS on 443 for
// the bundle Vite serves on 5173), the browser must reconnect HMR via
// the proxy port — otherwise it tries `wss://<host>:5173/` and the
// tunnel does not forward it. REVERIE_DEV_HMR_CLIENT_PORT carries that
// override; unset = localhost-style same-port reconnect.

export function parseHmrConfig(
  envValue: string | undefined,
): { hmr?: { clientPort: number } } {
  if (envValue === undefined || envValue.trim().length === 0) return {};
  const port = Number(envValue);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(
      `REVERIE_DEV_HMR_CLIENT_PORT must be an integer in 1..=65535, got ${JSON.stringify(envValue)}`,
    );
  }
  return { hmr: { clientPort: port } };
}
