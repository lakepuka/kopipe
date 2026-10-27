import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";

import { SourceModal } from "./SourceModal";

afterEach(cleanup);

// Provider 無しの描画なので既定言語（英語）の文言で検証する。
const SELECTION_BTN = "Paste selection only";

function select(textarea: HTMLTextAreaElement, start: number, end: number) {
  textarea.setSelectionRange(start, end);
  fireEvent.select(textarea);
}

describe("SourceModal 選択部分のみ貼り付け", () => {
  it("選択が無いうちは押せない", () => {
    render(<SourceModal text="hello world" html={null} onClose={() => {}} onPaste={() => {}} />);
    expect((screen.getByText(SELECTION_BTN) as HTMLButtonElement).disabled).toBe(true);
  });

  it("選択した範囲だけを貼り付けてモーダルを閉じる", () => {
    const onPaste = vi.fn();
    const onClose = vi.fn();
    render(<SourceModal text="hello world" html={null} onClose={onClose} onPaste={onPaste} />);
    select(screen.getByRole("textbox") as HTMLTextAreaElement, 6, 11);

    fireEvent.click(screen.getByText(SELECTION_BTN));
    expect(onPaste).toHaveBeenCalledWith("world");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("編集後の文字列から選択範囲を切り出す", () => {
    const onPaste = vi.fn();
    render(<SourceModal text="abc" html={null} onClose={() => {}} onPaste={onPaste} />);
    const area = screen.getByRole("textbox") as HTMLTextAreaElement;
    fireEvent.change(area, { target: { value: "abc def" } });
    select(area, 4, 7);

    fireEvent.click(screen.getByText(SELECTION_BTN));
    expect(onPaste).toHaveBeenCalledWith("def");
  });

  it("空白だけの選択は貼り付けない", () => {
    const onPaste = vi.fn();
    render(<SourceModal text="a  b" html={null} onClose={() => {}} onPaste={onPaste} />);
    select(screen.getByRole("textbox") as HTMLTextAreaElement, 1, 3);

    fireEvent.click(screen.getByText(SELECTION_BTN));
    expect(onPaste).not.toHaveBeenCalled();
  });

  it("タブを切り替えたら選択はリセットされる", () => {
    render(<SourceModal text="plain" html="<b>x</b>" onClose={() => {}} onPaste={() => {}} />);
    select(screen.getByRole("textbox") as HTMLTextAreaElement, 0, 3);
    expect((screen.getByText(SELECTION_BTN) as HTMLButtonElement).disabled).toBe(false);

    fireEvent.click(screen.getByText("HTML"));
    expect((screen.getByText(SELECTION_BTN) as HTMLButtonElement).disabled).toBe(true);
  });

  it("「貼り付け」は従来どおり全文を貼り付ける", () => {
    const onPaste = vi.fn();
    render(<SourceModal text="hello world" html={null} onClose={() => {}} onPaste={onPaste} />);
    fireEvent.click(screen.getByText("Paste"));
    expect(onPaste).toHaveBeenCalledWith("hello world");
  });
});
