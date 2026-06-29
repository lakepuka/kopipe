import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { type Clip, searchClips } from "../services/api";

// 1 ページの取得件数。スクロールで末尾に近づくと次ページを継ぎ足す（無限スクロール）。
const PAGE_SIZE = 100;

// 検索条件（query / ブックマークのみ / 正規表現）と結果一覧を管理するフック。
// 条件変更で先頭から再検索し、Rust の "clips-changed"（自動記録）にも追従する。
// loadMore() で続きを読み込み、hasMore が false になったら打ち止め。
export function useClips() {
  const [query, setQuery] = useState("");
  const [bookmarksOnly, setBookmarksOnly] = useState(false);
  const [useRegex, setUseRegex] = useState(false);
  const [clips, setClips] = useState<Clip[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [error, setError] = useState("");

  const runSeq = useRef(0);
  // loadMore は最新の値を同期的に見たいので ref に写しておく。
  const clipsRef = useRef<Clip[]>([]);
  const hasMoreRef = useRef(false);
  const loadingMore = useRef(false);

  const setClipsBoth = useCallback((next: Clip[]) => {
    clipsRef.current = next;
    setClips(next);
  }, []);
  const setHasMoreBoth = useCallback((v: boolean) => {
    hasMoreRef.current = v;
    setHasMore(v);
  }, []);

  // 先頭ページを取り直す（条件変更・自動記録・操作後）。
  const run = useCallback(
    async (q: string, marked: boolean, regex: boolean) => {
      const seq = ++runSeq.current;
      try {
        const result = await searchClips(q, marked, regex, PAGE_SIZE, 0);
        if (seq !== runSeq.current) return;
        setClipsBoth(result);
        setHasMoreBoth(result.length === PAGE_SIZE);
        setError("");
      } catch (e) {
        if (seq !== runSeq.current) return;
        setError(String(e));
      }
    },
    [setClipsBoth, setHasMoreBoth],
  );

  // 条件が変わるたびに先頭から検索（インクリメンタル検索）。
  useEffect(() => {
    run(query, bookmarksOnly, useRegex);
  }, [query, bookmarksOnly, useRegex, run]);

  // 最新条件を ref に保持し、購読は一度だけ張る。
  const latest = useRef({ query, bookmarksOnly, useRegex });
  latest.current = { query, bookmarksOnly, useRegex };

  useEffect(() => {
    const unlisten = listen("clips-changed", () => {
      const c = latest.current;
      run(c.query, c.bookmarksOnly, c.useRegex);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [run]);

  // 次ページを末尾に継ぎ足す。条件が途中で変わった場合（seq 不一致）は破棄。
  const loadMore = useCallback(async () => {
    if (loadingMore.current || !hasMoreRef.current) return;
    loadingMore.current = true;
    const seq = runSeq.current;
    const c = latest.current;
    const offset = clipsRef.current.length;
    try {
      const more = await searchClips(c.query, c.bookmarksOnly, c.useRegex, PAGE_SIZE, offset);
      if (seq !== runSeq.current) return;
      setClipsBoth([...clipsRef.current, ...more]);
      setHasMoreBoth(more.length === PAGE_SIZE);
    } catch (e) {
      if (seq === runSeq.current) setError(String(e));
    } finally {
      loadingMore.current = false;
    }
  }, [setClipsBoth, setHasMoreBoth]);

  // 操作後に今の条件で先頭から引き直す。
  function refresh() {
    const c = latest.current;
    return run(c.query, c.bookmarksOnly, c.useRegex);
  }

  return {
    query,
    setQuery,
    bookmarksOnly,
    setBookmarksOnly,
    useRegex,
    setUseRegex,
    clips,
    hasMore,
    loadMore,
    error,
    setError,
    refresh,
  };
}
