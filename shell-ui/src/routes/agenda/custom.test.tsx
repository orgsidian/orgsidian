import { describe, expect, it, vi } from "vitest";

/**
 * Story 7.4 (LD-29): `isoDateOrUndefined` is the route's `validateSearch`
 * guard for `?start=` / `?end=`. It accepts only a real `YYYY-MM-DD` calendar
 * day and rejects shape-valid-but-impossible dates from a hand-edited URL
 * (e.g. `2026-13-45`), which `new Date` would otherwise silently roll over
 * into a different day and forward to the query. These unit-test that guard
 * directly (the component/route tests exercise the wired path).
 */

// The route module calls `createFileRoute("/agenda/custom")(...)` at import
// time and pulls in the heavy component; neither is under test here, so mock
// the router factory (no generated route tree) and stub the component.
vi.mock("@tanstack/react-router", () => ({
  createFileRoute: () => (opts: unknown) => ({ options: opts }),
}));
vi.mock("@/components/agenda/AgendaCustom", () => ({
  AgendaCustom: () => null,
}));

// Imported AFTER the mocks are registered.
import { isoDateOrUndefined } from "./custom";

describe("isoDateOrUndefined (Story 7.4, LD-29)", () => {
  it("passes a real calendar date through unchanged", () => {
    expect(isoDateOrUndefined("2026-09-05")).toBe("2026-09-05");
    expect(isoDateOrUndefined("2024-02-29")).toBe("2024-02-29"); // leap day
  });

  it("rejects shape-valid but impossible calendar dates", () => {
    expect(isoDateOrUndefined("2026-13-45")).toBeUndefined();
    expect(isoDateOrUndefined("2026-02-30")).toBeUndefined();
    expect(isoDateOrUndefined("2026-00-10")).toBeUndefined();
    expect(isoDateOrUndefined("2025-02-29")).toBeUndefined(); // not a leap year
  });

  it("rejects malformed shapes and non-strings", () => {
    expect(isoDateOrUndefined("2026-9-5")).toBeUndefined();
    expect(isoDateOrUndefined("09-05-2026")).toBeUndefined();
    expect(isoDateOrUndefined("not-a-date")).toBeUndefined();
    expect(isoDateOrUndefined("")).toBeUndefined();
    expect(isoDateOrUndefined(undefined)).toBeUndefined();
    expect(isoDateOrUndefined(20260905)).toBeUndefined();
  });
});
