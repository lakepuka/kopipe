import { describe, expect, it } from "vitest";

import { isNewerVersion } from "./version";

describe("isNewerVersion", () => {
  it("detects a newer version", () => {
    expect(isNewerVersion("0.2.0", "0.1.0")).toBe(true);
    expect(isNewerVersion("1.0.0", "0.9.9")).toBe(true);
    expect(isNewerVersion("0.1.1", "0.1.0")).toBe(true);
  });

  it("returns false for same or older", () => {
    expect(isNewerVersion("0.1.0", "0.1.0")).toBe(false);
    expect(isNewerVersion("0.1.0", "0.2.0")).toBe(false);
    expect(isNewerVersion("1.0.0", "1.0.1")).toBe(false);
  });

  it("ignores a leading v and uneven lengths", () => {
    expect(isNewerVersion("v0.2.0", "0.1.0")).toBe(true);
    expect(isNewerVersion("v1.2", "1.2.0")).toBe(false);
    expect(isNewerVersion("1.2.1", "v1.2")).toBe(true);
  });
});
