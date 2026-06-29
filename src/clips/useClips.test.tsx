import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";

import type { Clip } from "../services/api";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("../services/api", () => ({
  searchClips: vi.fn(),
}));

import { searchClips } from "../services/api";
import { useClips } from "./useClips";

const mockSearchClips = vi.mocked(searchClips);

const clip = (id: number, content: string): Clip => ({
  id,
  content,
  created_at: 0,
  bookmark: false,
  kind: "text",
  image_path: null,
  html: null,
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

afterEach(() => {
  cleanup();
  mockSearchClips.mockReset();
});

describe("useClips", () => {
  it("古い検索レスポンスで新しい結果を上書きしない", async () => {
    const first = deferred<Clip[]>();
    const second = deferred<Clip[]>();
    mockSearchClips.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);

    const { result } = renderHook(() => useClips());

    act(() => {
      result.current.setQuery("new");
    });

    await waitFor(() => expect(mockSearchClips).toHaveBeenCalledTimes(2));

    await act(async () => {
      second.resolve([clip(2, "new")]);
      await second.promise;
    });
    expect(result.current.clips).toEqual([clip(2, "new")]);

    await act(async () => {
      first.resolve([clip(1, "old")]);
      await first.promise;
    });
    expect(result.current.clips).toEqual([clip(2, "new")]);
  });
});
