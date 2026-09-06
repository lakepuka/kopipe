import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { listTags } from "../services/api";

// 使用中のタグ一覧（名前順）を保持するフック。
// タグの増減は履歴の変化と同じ経路（clips-changed）で起きるので、それに追従する。
// どこからも参照されなくなったタグは Rust 側で消えるため、ここには出てこない。
export function useTags() {
  const [tags, setTags] = useState<string[]>([]);

  const refresh = useCallback(async () => {
    try {
      setTags(await listTags());
    } catch {
      // 一覧が引けなくても履歴の表示は続けたいので握りつぶす。
    }
  }, []);

  useEffect(() => {
    refresh();
    const unlisten = listen("clips-changed", refresh);
    return () => {
      unlisten.then((f) => f());
    };
  }, [refresh]);

  return { tags, refresh };
}
