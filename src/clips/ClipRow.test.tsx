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
};
const imageClip: Clip = {
  id: 2,
  content: "",
  created_at: 0,
  bookmark: true,
  kind: "image",
  image_path: "a.png",
  html: null,
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

  it("copied=true で『Copied!』表示", () => {
    render(<ClipRow {...base} clip={textClip} copied />);
    expect(screen.getByText("Copied!")).toBeTruthy();
  });
});
