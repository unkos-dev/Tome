// Declarative loopback defaults used when REVERIE_DEV_HOSTS is unset.
//
// Vite's host-validation middleware (`isHostAllowedInternal` in
// `vite/dist/node/chunks/node.js`) short-circuits *before* iterating the
// allowlist for any IPv4 / IPv6 literal Host header and for any hostname
// equal to `localhost` or ending in `.localhost`. As a result these three
// entries are functionally no-ops at runtime — Vite would accept loopback
// requests even with an empty allowlist. They remain here as a declared
// scope (and the IPv6 literal is in Vite's bracket-stripped form, so it
// would match if Vite's hardcoded ipv6 short-circuit is ever removed).
// The DNS-rebind guard only filters *non-loopback hostnames* the developer
// explicitly adds via REVERIE_DEV_HOSTS.
export const DEFAULT_LOOPBACK_HOSTS: readonly string[] = [
  "localhost",
  "127.0.0.1",
  "[::1]",
];

export function parseAllowedHosts(envValue: string | undefined): string[] {
  if (envValue === undefined) return [...DEFAULT_LOOPBACK_HOSTS];
  const entries = envValue
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
  if (entries.length === 0) return [...DEFAULT_LOOPBACK_HOSTS];
  return entries;
}
