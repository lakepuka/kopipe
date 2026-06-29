import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";

import type { Clip } from "../services/api";
import { ClipMenu } from "./ClipMenu";

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
  x: 10,
  y: 10,
  onView: noop,
  onCopy: noop,
  onPastePlain: noop,
  onToggleBookmark: noop,
  onDelete: noop,
  onClose: noop,
};

// Provider 無しの描画なので既定言語（英語）の文言で検証する。
describe("ClipMenu", () => {
  it("テキスト: コピー/ブックマーク追加/削除、クリックでコールバック", () => {
    const onCopy = vi.fn();
    const onDelete = vi.fn();
    render(<ClipMenu {...base} clip={textClip} onCopy={onCopy} onDelete={onDelete} />);

    expect(screen.getByText("View")).toBeTruthy();
    expect(screen.getByText("Copy")).toBeTruthy();
    expect(screen.getByText("Add bookmark")).toBeTruthy();

    fireEvent.click(screen.getByText("Copy"));
    fireEvent.click(screen.getByText("Delete"));
    expect(onCopy).toHaveBeenCalledTimes(1);
    expect(onDelete).toHaveBeenCalledTimes(1);
  });

  it("画像: 表示あり・コピーは無し・ブックマークラベルは解除", () => {
    render(<ClipMenu {...base} clip={imageClip} />);
    expect(screen.getByText("View")).toBeTruthy();
    expect(screen.queryByText("Copy")).toBeNull();
    expect(screen.getByText("Remove bookmark")).toBeTruthy();
  });
});
