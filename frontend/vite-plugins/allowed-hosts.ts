// Loopback allowlist used when REVERIE_DEV_HOSTS is unset. Tightest reasonable
// default: a developer running `npm run dev` on their workstation can reach
// the bundle via these hosts; nothing else can. Cloud dev environments
// (Coder, Codespaces) must export REVERIE_DEV_HOSTS explicitly.
export const DEFAULT_LOOPBACK_HOSTS: readonly string[] = [
  "localhost",
  "127.0.0.1",
  "::1",
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
