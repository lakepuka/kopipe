import { describe, expect, it } from "vitest";

import { parseMaxLines, parsePinned, parseResetSearchOnShow, parseShowIcons } from "./prefs";

// 未設定のときにどちらへ倒れるかが仕様なので、そこを固定する。
describe("設定のパースと既定値", () => {
  it("行アイコンは未設定なら表示（オプトアウト式）", () => {
    expect(parseShowIcons(undefined)).toBe(true);
    expect(parseShowIcons("false")).toBe(false);
  });

  it("最大行数は未設定なら 1、0 は「全行」としてそのまま", () => {
    expect(parseMaxLines(undefined)).toBe(1);
    expect(parseMaxLines("あ")).toBe(1);
    expect(parseMaxLines("0")).toBe(0);
  });

  it("ピン留めは未設定ならオフ", () => {
    expect(parsePinned(undefined)).toBe(false);
    expect(parsePinned("true")).toBe(true);
  });

  it("呼び出し時の検索リセットは未設定ならオフ", () => {
    expect(parseResetSearchOnShow(undefined)).toBe(false);
    expect(parseResetSearchOnShow("false")).toBe(false);
    expect(parseResetSearchOnShow("true")).toBe(true);
  });
});
