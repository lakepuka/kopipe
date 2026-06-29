import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";

import { ClipText } from "./ClipText";

// 各テスト後に DOM を片付ける（描画が次のテストへ漏れないように）。
afterEach(cleanup);

describe("ClipText", () => {
  // Provider 無しの描画なので既定言語（英語）の文言で検証する。
  it("内容を表示し、展開トグルは出さない", () => {
    render(<ClipText content="hello" onPaste={() => {}} />);
    expect(screen.getByText("hello")).toBeTruthy();
    expect(screen.queryByTitle("Show full text")).toBeNull();
    expect(screen.queryByTitle("Collapse")).toBeNull();
  });

  it("クリックで onPaste が呼ばれる", () => {
    const onPaste = vi.fn();
    render(<ClipText content="hello" onPaste={onPaste} />);
    fireEvent.click(screen.getByText("hello"));
    expect(onPaste).toHaveBeenCalledTimes(1);
  });
});
