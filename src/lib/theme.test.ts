import { describe, expect, it } from "vitest";

import { DEFAULT_THEME, parseTheme } from "./theme";

describe("parseTheme", () => {
  it("既知のテーマ名はそのまま返す", () => {
    expect(parseTheme("kopipe")).toBe("kopipe");
    expect(parseTheme("light")).toBe("light");
    expect(parseTheme("dark")).toBe("dark");
    expect(parseTheme("mint")).toBe("mint");
    expect(parseTheme("grape")).toBe("grape");
    expect(parseTheme("sky")).toBe("sky");
    expect(parseTheme("system")).toBe("system");
  });

  it("未知・空・null は既定テーマにフォールバックする", () => {
    expect(parseTheme("")).toBe(DEFAULT_THEME);
    expect(parseTheme("blue")).toBe(DEFAULT_THEME);
    expect(parseTheme(null)).toBe(DEFAULT_THEME);
    expect(parseTheme(undefined)).toBe(DEFAULT_THEME);
  });
});
