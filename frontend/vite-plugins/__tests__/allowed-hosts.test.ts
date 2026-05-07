import { describe, expect, it } from "vitest";

import {
  DEFAULT_LOOPBACK_HOSTS,
  parseAllowedHosts,
} from "../allowed-hosts";

describe("parseAllowedHosts", () => {
  it("returns loopback defaults when env var is undefined", () => {
    expect(parseAllowedHosts(undefined)).toEqual(DEFAULT_LOOPBACK_HOSTS);
  });

  it("returns loopback defaults when env var is empty string", () => {
    expect(parseAllowedHosts("")).toEqual(DEFAULT_LOOPBACK_HOSTS);
  });

  it("returns loopback defaults when env var is whitespace only", () => {
    expect(parseAllowedHosts("   \t\n")).toEqual(DEFAULT_LOOPBACK_HOSTS);
  });

  it("parses a single host", () => {
    expect(parseAllowedHosts("dev.reverie.unkos.net")).toEqual([
      "dev.reverie.unkos.net",
    ]);
  });

  it("parses a comma-separated list", () => {
    expect(parseAllowedHosts("a.example,b.example,c.example")).toEqual([
      "a.example",
      "b.example",
      "c.example",
    ]);
  });

  it("trims surrounding whitespace from each entry", () => {
    expect(parseAllowedHosts(" a.example ,  b.example\t,c.example ")).toEqual([
      "a.example",
      "b.example",
      "c.example",
    ]);
  });

  it("drops empty entries from doubled or trailing commas", () => {
    expect(parseAllowedHosts("a.example,,b.example,")).toEqual([
      "a.example",
      "b.example",
    ]);
  });

  it("does not merge user list with loopback defaults", () => {
    // Replace, not merge — caller decides whether loopback access is needed.
    const result = parseAllowedHosts("dev.reverie.unkos.net");
    expect(result).not.toContain("localhost");
    expect(result).not.toContain("127.0.0.1");
  });

  it("DEFAULT_LOOPBACK_HOSTS is exactly the loopback set", () => {
    expect(DEFAULT_LOOPBACK_HOSTS).toEqual(["localhost", "127.0.0.1", "::1"]);
  });
});
