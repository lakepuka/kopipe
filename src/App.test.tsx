import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";

// Tauri / プラグインの API はネイティブ依存なのでモックする。これは E2E ではなく
// 「主画面 UI がクラッシュせず描画される」ことを確かめるスモークテスト。
vi.mock("@tauri-apps/api/core", () => ({
  // search_clips は配列、それ以外（get_settings 等）は設定レコードを返す。
  invoke: vi.fn(async (cmd: string) =>
    cmd === "search_clips" ? [] : { autostart_prompted: "true" },
  ),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    hide: vi.fn(),
    onFocusChanged: vi.fn(() => Promise.resolve(() => {})),
  }),
}));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  writeText: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-autostart", () => ({
  enable: vi.fn(),
  disable: vi.fn(),
  isEnabled: vi.fn(() => Promise.resolve(false)),
}));

import App from "./App";

beforeEach(() => {
  // jsdom には matchMedia が無いので最小実装を入れる（テーマ追従で使う）。
  vi.stubGlobal(
    "matchMedia",
    vi.fn().mockReturnValue({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }),
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("App (smoke)", () => {
  it("クラッシュせず主画面（上部バー）が描画される", () => {
    render(<App />);
    expect(screen.getByText("kopipe")).toBeTruthy();
  });
});
