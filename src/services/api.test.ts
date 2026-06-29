import { afterEach, describe, expect, it, vi } from "vitest";

// Tauri の invoke / clipboard をモックして、api が正しいコマンド・引数で呼ぶかを検証する。
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve()),
}));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  writeText: vi.fn(() => Promise.resolve()),
}));

import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

import {
  appVersion,
  clearClips,
  type Clip,
  copyClip,
  imageDirPath,
  imageStorageBytes,
  pasteClip,
  pasteText,
  resetAppSettings,
  searchClips,
  setImageLimit,
  setPinned,
  setShortcut,
} from "./api";

const mockInvoke = vi.mocked(invoke);
const mockWriteText = vi.mocked(writeText);
afterEach(() => {
  mockInvoke.mockClear();
  mockWriteText.mockClear();
});

const text: Clip = {
  id: 1,
  content: "x",
  created_at: 0,
  bookmark: false,
  kind: "text",
  image_path: null,
  html: null,
};
const image: Clip = { ...text, id: 2, kind: "image", image_path: "a.png" };
const files: Clip = { ...text, id: 3, kind: "files", content: "C:/a\nC:/b" };
const htmlClip: Clip = { ...text, id: 4, html: "<b>x</b>" };

describe("api", () => {
  it("searchClips は camelCase 引数で search_clips を呼ぶ", () => {
    searchClips("foo bar", true, false);
    expect(mockInvoke).toHaveBeenCalledWith("search_clips", {
      query: "foo bar",
      limit: 100,
      offset: 0,
      bookmarksOnly: true,
      useRegex: false,
    });
  });

  it("pasteClip はテキストなら paste_clip、画像なら paste_image を呼ぶ", () => {
    pasteClip(text);
    expect(mockInvoke).toHaveBeenCalledWith("paste_clip", { content: "x", html: null });

    mockInvoke.mockClear();
    pasteClip(image);
    expect(mockInvoke).toHaveBeenCalledWith("paste_image", { id: 2 });

    mockInvoke.mockClear();
    pasteClip(files);
    expect(mockInvoke).toHaveBeenCalledWith("paste_files", {
      paths: ["C:/a", "C:/b"],
    });
  });

  it("pasteClip は HTML 付きならその html を渡す", () => {
    pasteClip(htmlClip);
    expect(mockInvoke).toHaveBeenCalledWith("paste_clip", {
      content: "x",
      html: "<b>x</b>",
    });
  });

  it("pasteText は常にプレーン(html=null)で paste_clip を呼ぶ", () => {
    pasteText("hello");
    expect(mockInvoke).toHaveBeenCalledWith("paste_clip", {
      content: "hello",
      html: null,
    });
  });

  it("copyClip は files=実ファイル / html=両形式 / それ以外=テキスト", () => {
    copyClip(files);
    expect(mockInvoke).toHaveBeenCalledWith("copy_files", {
      paths: ["C:/a", "C:/b"],
    });

    mockInvoke.mockClear();
    copyClip(htmlClip);
    expect(mockInvoke).toHaveBeenCalledWith("copy_text", {
      content: "x",
      html: "<b>x</b>",
    });

    copyClip(text);
    expect(mockWriteText).toHaveBeenCalledWith("x");
  });

  it("設定画面向けの command wrapper は正しいコマンド名と引数で invoke する", () => {
    imageStorageBytes();
    expect(mockInvoke).toHaveBeenCalledWith("image_storage_bytes");

    imageDirPath();
    expect(mockInvoke).toHaveBeenCalledWith("image_dir_path");

    appVersion();
    expect(mockInvoke).toHaveBeenCalledWith("app_version");

    setShortcut("Control+KeyK");
    expect(mockInvoke).toHaveBeenCalledWith("set_shortcut", { accelerator: "Control+KeyK" });

    setImageLimit(256);
    expect(mockInvoke).toHaveBeenCalledWith("set_image_limit", { mb: 256 });

    clearClips("image");
    expect(mockInvoke).toHaveBeenCalledWith("clear_clips", { kind: "image" });

    resetAppSettings();
    expect(mockInvoke).toHaveBeenCalledWith("reset_settings");

    setPinned(true);
    expect(mockInvoke).toHaveBeenCalledWith("set_pinned", { pinned: true });
  });
});
