import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";

// ClipImage が叩く invoke をモック（画像行でも描画が落ちないように）。
vi.mock("@tauri-apps/api/core", () => ({
  invoke: () => Promise.resolve(""),
}));

import type { Clip } from "../services/api";
import { ClipRow } from "./ClipRow";

afterEach(cleanup);

const textClip: Clip = {
  id: 1,
  content: "hello",
  created_at: 0,
  bookmark: false,
  kind: "text",
  image_path: null,
  html: null,
  note: null,
  tags: [],
};
const imageClip: Clip = {
  id: 2,
  content: "",
  created_at: 0,
  bookmark: true,
  kind: "image",
  image_path: "a.png",
  html: null,
  note: null,
  tags: [],
};

const noop = () => {};
const base = {
  copied: false,
  showBadge: true,
  maxLines: 1,
  onPaste: noop,
  onInspect: noop,
  onRequestMenu: noop,
};

// Provider 無しの描画なので既定言語（英語）の文言で検証する。
describe("ClipRow", () => {
  it("テキスト行: TXT バッジと詳細ボタンあり・プレビューは無し", () => {
    render(<ClipRow {...base} clip={textClip} />);
    expect(screen.getByText("TXT")).toBeTruthy();
    expect(screen.getByTitle("Details")).toBeTruthy();
    expect(screen.queryByTitle("Preview")).toBeNull();
  });

  it("画像行: IMG バッジ（拡大プレビュー）あり", () => {
    render(<ClipRow {...base} clip={imageClip} />);
    expect(screen.getByText("IMG")).toBeTruthy();
    expect(screen.getByTitle("Preview")).toBeTruthy();
  });

  it("IMG バッジのクリックで onInspect が呼ばれる", () => {
    const onInspect = vi.fn();
    render(<ClipRow {...base} clip={imageClip} onInspect={onInspect} />);
    fireEvent.click(screen.getByText("IMG"));
    expect(onInspect).toHaveBeenCalledTimes(1);
  });

  it("⋮ クリックで onRequestMenu が呼ばれる", () => {
    const onRequestMenu = vi.fn();
    render(<ClipRow {...base} clip={textClip} onRequestMenu={onRequestMenu} />);
    fireEvent.click(screen.getByTitle("Details"));
    expect(onRequestMenu).toHaveBeenCalledTimes(1);
  });

  it("説明とタグを右側に表示する（説明は tooltip で欄名が分かる）", () => {
    render(
      <ClipRow {...base} clip={{ ...textClip, note: "ブックマーク抽出", tags: ["仕事", "SQL"] }} />,
    );
    expect(screen.getByText("仕事")).toBeTruthy();
    expect(screen.getByText("SQL")).toBeTruthy();
    expect(screen.getByTitle("Note: ブックマーク抽出")).toBeTruthy();
  });

  it("タグが 3 つ以上なら 2 つ＋残り件数にまとめる", () => {
    render(<ClipRow {...base} clip={{ ...textClip, tags: ["a", "b", "c", "d"] }} />);
    expect(screen.getByText("a")).toBeTruthy();
    expect(screen.getByText("b")).toBeTruthy();
    expect(screen.queryByText("c")).toBeNull();
    expect(screen.getByText("+2")).toBeTruthy();
  });

  it("説明もタグも無ければ右側の枠ごと描画しない（幅を取らない）", () => {
    const { container } = render(<ClipRow {...base} clip={textClip} />);
    expect(container.querySelector(".clip-meta")).toBeNull();
  });

  it("copied=true で『Copied!』表示", () => {
    render(<ClipRow {...base} clip={textClip} copied />);
    expect(screen.getByText("Copied!")).toBeTruthy();
  });
});
