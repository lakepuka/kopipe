import { describe, expect, it } from "vitest";

import { parseTagQuery, toggleTagInQuery } from "./tagQuery";

describe("parseTagQuery", () => {
  it("tag: が無ければ全部が本文検索", () => {
    expect(parseTagQuery("請求書 控え")).toEqual({ text: "請求書 控え", tags: [] });
  });

  it("tag: を切り出し、残りを本文検索にする", () => {
    expect(parseTagQuery("請求書 tag:仕事,SQL 控え")).toEqual({
      text: "請求書 控え",
      tags: ["仕事", "SQL"],
    });
  });

  it("複数の tag: トークンをまとめ、大小違いは 1 つ扱い", () => {
    expect(parseTagQuery("TAG:a tag:A,b").tags).toEqual(["a", "b"]);
  });

  it("空のタグ名は無視する", () => {
    expect(parseTagQuery("tag: tag:,x,").tags).toEqual(["x"]);
  });
});

describe("toggleTagInQuery", () => {
  it("空の検索に足すと tag: だけになる", () => {
    expect(toggleTagInQuery("", "仕事")).toBe("tag:仕事");
  });

  it("2 つ目はカンマで足す", () => {
    expect(toggleTagInQuery("tag:仕事", "SQL")).toBe("tag:仕事,SQL");
  });

  it("既にある文字列は残し、末尾に tag: を付ける", () => {
    expect(toggleTagInQuery("請求書", "仕事")).toBe("請求書 tag:仕事");
    expect(toggleTagInQuery("請求書 ", "仕事")).toBe("請求書 tag:仕事");
  });

  it("もう一度押すと外れ、最後の 1 つならトークンごと消える", () => {
    expect(toggleTagInQuery("tag:仕事,SQL", "仕事")).toBe("tag:SQL");
    expect(toggleTagInQuery("請求書 tag:仕事", "仕事")).toBe("請求書");
  });

  it("大小文字が違っても同じタグとして外せる", () => {
    expect(toggleTagInQuery("tag:SQL", "sql")).toBe("");
  });
});
