import { describe, expect, it } from "vitest";

import { buildAccelerator, describeShortcut, formatAccelerator } from "./shortcut";

const base = { ctrlKey: false, altKey: false, shiftKey: false, metaKey: false };

describe("buildAccelerator", () => {
  it("修飾キー＋通常キーをアクセラレータにする", () => {
    expect(buildAccelerator({ ...base, ctrlKey: true, shiftKey: true, code: "KeyV" })).toBe(
      "Control+Shift+KeyV",
    );
    expect(buildAccelerator({ ...base, altKey: true, code: "Digit1" })).toBe("Alt+Digit1");
  });

  it("修飾キー単体は未確定(null)", () => {
    expect(buildAccelerator({ ...base, ctrlKey: true, code: "ControlLeft" })).toBeNull();
  });

  it("修飾キー無しは不可(null)", () => {
    expect(buildAccelerator({ ...base, code: "KeyV" })).toBeNull();
  });
});

describe("formatAccelerator", () => {
  it("読みやすい表記に変換する", () => {
    expect(formatAccelerator("Control+Shift+KeyV")).toBe("Ctrl + Shift + V");
    expect(formatAccelerator("Alt+Digit1")).toBe("Alt + 1");
    expect(formatAccelerator("Super+KeyK")).toBe("Win + K");
  });
});

describe("describeShortcut", () => {
  it("2回押しは専用表記、コンボは整形表記", () => {
    expect(describeShortcut("double:Shift")).toBe("Shift 2回");
    expect(describeShortcut("double:Control")).toBe("Ctrl 2回");
    expect(describeShortcut("Control+Shift+KeyV")).toBe("Ctrl + Shift + V");
  });
});
