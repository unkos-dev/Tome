// When the dev server is fronted by a reverse proxy on a different
// external port (e.g. a Cloudflare tunnel terminating TLS on 443 for
// the bundle Vite serves on 5173), the browser must reconnect HMR via
// the proxy port — otherwise it tries `wss://<host>:5173/` and the
// tunnel does not forward it. REVERIE_DEV_HMR_CLIENT_PORT carries that
// override value.
/**
 * Parse the REVERIE_DEV_HMR_CLIENT_PORT environment value into a Vite HMR client port configuration.
 *
 * If `envValue` is `undefined` or blank, the function treats the setting as unset and returns an empty object.
 *
 * @param envValue - The raw value of `REVERIE_DEV_HMR_CLIENT_PORT` (may be `undefined` or whitespace)
 * @returns An object containing `hmr.clientPort` when `envValue` is a valid port; otherwise an empty object
 * @throws Error if `envValue` cannot be parsed to an integer in the range 1..=65535. The error message includes the original value JSON-encoded.
 */

export function parseHmrConfig(
  envValue: string | undefined,
): { hmr?: { clientPort: number } } {
  if (envValue === undefined || envValue.trim().length === 0) return {};
  const trimmed = envValue.trim();
  const invalid = new Error(
    `REVERIE_DEV_HMR_CLIENT_PORT must be an integer in 1..=65535, got ${JSON.stringify(envValue)}`,
  );
  if (!/^\d+$/.test(trimmed)) throw invalid;
  const port = Number(trimmed);
  if (port < 1 || port > 65535) throw invalid;
  return { hmr: { clientPort: port } };
}
