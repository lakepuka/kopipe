import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { ClipMenu } from "./clips/ClipMenu";
import { ClipMetaPanel } from "./clips/ClipMetaPanel";
import { ClipRow } from "./clips/ClipRow";
import { PreviewModal } from "./clips/PreviewModal";
import { SourceModal } from "./clips/SourceModal";
import { useClips } from "./clips/useClips";
import { useTags } from "./clips/useTags";
import { useT } from "./i18n";
import { getSettings, loadDisplay, parseUpdateCheck, SETTING_KEYS } from "./lib/prefs";
import { parseTagQuery, toggleTagInQuery } from "./lib/tagQuery";
import { applyTheme, loadTheme } from "./lib/theme";
import { isNewerVersion } from "./lib/version";
import { Onboarding } from "./components/Onboarding";
import { TopBar } from "./components/TopBar";
import {
  appVersion,
  type Clip,
  copyClip,
  deleteClip,
  fetchLatestRelease,
  filePaths,
  imageDataUrl,
  type LatestRelease,
  openExternal,
  pasteClip,
  pasteText,
  revealPath,
  toggleBookmark,
} from "./services/api";

type MenuState = { clip: Clip; x: number; y: number };
type MetaState = { clip: Clip; x: number; y: number; focus: "note" | "tags" };
type SourceState = { text: string; html: string | null };

function App() {
  const t = useT();
  const search = useClips();
  const tags = useTags();
  const [copiedId, setCopiedId] = useState<number | null>(null);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [meta, setMeta] = useState<MetaState | null>(null);
  const [previewSrc, setPreviewSrc] = useState<string | null>(null);
  const [source, setSource] = useState<SourceState | null>(null);
  const [showIcons, setShowIcons] = useState(true);
  const [maxLines, setMaxLines] = useState(1);
  const [pinned, setPinned] = useState(false);
  // 呼び出しのたびに検索条件を戻すか（既定はオフ＝前回の絞り込みを保つ）。
  // main-shown のリスナは張り替えたくないので、値は ref で見る。
  const resetOnShow = useRef(false);
  const [onboarding, setOnboarding] = useState(false);
  const [update, setUpdate] = useState<LatestRelease | null>(null);

  // Esc で閉じる。モーダル／メニューが開いていればそれを先に閉じ、無ければ
  // ウィンドウを隠す（X ボタンと同じ＝破棄せず再利用）。
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      // オンボーディング中は Esc でウィンドウを隠さない（明示的に進めてもらう）。
      if (onboarding) return;
      if (source) setSource(null);
      else if (previewSrc) setPreviewSrc(null);
      // 編集パネルの Esc は「保存せず閉じる」（外クリックや Enter は保存）。
      else if (meta) setMeta(null);
      else if (menu) setMenu(null);
      else getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onboarding, source, previewSrc, menu, meta]);

  // ↑↓ で履歴リストを移動する。検索欄や本体にフォーカスがある状態で ↓ を押すと
  // 先頭行へ、先頭行で ↑ を押すと検索欄へ戻る。Enter でのペーストは各行側で処理。
  useEffect(() => {
    const onArrow = (e: KeyboardEvent) => {
      if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
      // モーダル／メニュー／オンボーディング表示中はリスト移動しない。
      if (onboarding || menu || meta || previewSrc || source) return;
      const rows = Array.from(document.querySelectorAll<HTMLElement>(".clip-row"));
      if (rows.length === 0) return;
      const active = document.activeElement as HTMLElement | null;
      const idx = active ? rows.indexOf(active) : -1;

      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (idx === -1) rows[0].focus();
        else if (idx < rows.length - 1) rows[idx + 1].focus();
      } else {
        // ArrowUp
        if (idx > 0) {
          e.preventDefault();
          rows[idx - 1].focus();
        } else if (idx === 0) {
          e.preventDefault();
          document.querySelector<HTMLElement>(".search")?.focus();
        }
        // 検索欄など（idx === -1）ではキャレット移動を邪魔しない。
      }
    };
    window.addEventListener("keydown", onArrow);
    return () => window.removeEventListener("keydown", onArrow);
  }, [onboarding, menu, meta, previewSrc, source]);

  // ウィンドウがフォーカスを得た直後の最初のクリックは「アクティブ化のためのクリック」とみなし、
  // 行クリック等に伝えない。設定ウィンドウを出した状態でメインをクリックしたときは、
  // 設定が閉じるだけで、メイン側は押されたことにならない。
  // ※ピン留め中は前面のまま 1 クリックでペーストしたいので、この抑止は行わない。
  useEffect(() => {
    if (pinned) return;
    let activatedAt = 0;
    const onFocus = () => {
      activatedAt = Date.now();
    };
    const onClickCapture = (e: MouseEvent) => {
      if (Date.now() - activatedAt < 250) {
        e.stopPropagation();
        e.preventDefault();
        activatedAt = 0;
      }
    };
    window.addEventListener("focus", onFocus);
    window.addEventListener("click", onClickCapture, true);
    return () => {
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("click", onClickCapture, true);
    };
  }, [pinned]);

  // テーマ・表示設定を起動時に適用し、設定変更／OS のダーク切替（system 時）にも追従する。
  useEffect(() => {
    const apply = () => {
      loadTheme().then(applyTheme);
      loadDisplay().then((d) => {
        setShowIcons(d.showIcons);
        setMaxLines(d.maxLines);
        setPinned(d.pinned);
        resetOnShow.current = d.resetSearchOnShow;
      });
    };
    apply();
    const unlisten = listen("settings-changed", apply);
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener("change", apply);
    return () => {
      unlisten.then((f) => f());
      mq.removeEventListener("change", apply);
    };
  }, []);

  // 呼び出し（トレイ・ショートカット・2 回押し）のたびに検索条件を戻す。設定が ON のときだけ。
  // 正規表現モードは条件ではなく「検索のしかた」なので残す。
  useEffect(() => {
    const unlisten = listen("main-shown", () => {
      if (!resetOnShow.current) return;
      search.setQuery(""); // tag: の絞り込みも検索文字列に入っているのでこれで消える
      search.setBookmarksOnly(false);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [search.setQuery, search.setBookmarksOnly]);

  // 初回起動時のみ：オンボーディング（言語→ようこそ→自動起動）を表示する。
  useEffect(() => {
    getSettings().then((s) => {
      if (s[SETTING_KEYS.autostartPrompted] !== "true") setOnboarding(true);
    });
  }, []);

  // 起動時に一度だけ更新確認（設定で無効なら何もしない）。kopipe で唯一の外部通信。
  useEffect(() => {
    getSettings().then((s) => {
      if (!parseUpdateCheck(s[SETTING_KEYS.updateCheck])) return;
      fetchLatestRelease().then((rel) => {
        if (!rel) return;
        appVersion().then((cur) => {
          if (isNewerVersion(rel.tag, cur)) setUpdate(rel);
        });
      });
    });
  }, []);

  async function handleCopy(c: Clip) {
    try {
      await copyClip(c);
      setMenu(null);
      setCopiedId(c.id);
      setTimeout(() => setCopiedId((cur) => (cur === c.id ? null : cur)), 1200);
    } catch (e) {
      search.setError(String(e));
    }
  }

  async function handlePaste(c: Clip) {
    try {
      await pasteClip(c);
    } catch (e) {
      search.setError(String(e));
    }
  }

  // 種別によらず content をプレーン文字として貼り付ける。
  async function handlePastePlain(c: Clip) {
    setMenu(null);
    try {
      await pasteText(c.content);
    } catch (e) {
      search.setError(String(e));
    }
  }

  async function handleToggleBookmark(id: number) {
    try {
      await toggleBookmark(id);
      setMenu(null);
      await search.refresh();
    } catch (e) {
      search.setError(String(e));
    }
  }

  async function handleDelete(id: number) {
    try {
      await deleteClip(id);
      setMenu(null);
      await search.refresh();
    } catch (e) {
      search.setError(String(e));
    }
  }

  // 先頭バッジのクリック。種別ごとに中身を確認する。
  async function handleInspect(c: Clip) {
    try {
      if (c.kind === "image") {
        setPreviewSrc(await imageDataUrl(c.id));
      } else if (c.kind === "files") {
        const first = filePaths(c)[0];
        if (first) await revealPath(first);
      } else {
        setSource({ text: c.content, html: c.html });
      }
    } catch (e) {
      search.setError(String(e));
    }
  }

  return (
    <main className="container">
      {update && (
        <div className="update-banner">
          <span>
            {t("update_available")} {update.tag}
          </span>
          <span className="update-spacer" />
          <button type="button" className="update-get" onClick={() => openExternal(update.url)}>
            {t("update_get")}
          </button>
          <button
            type="button"
            className="icon-btn"
            onClick={() => setUpdate(null)}
            aria-label={t("close")}
          >
            ✕
          </button>
        </div>
      )}

      <TopBar
        query={search.query}
        onQueryChange={search.setQuery}
        useRegex={search.useRegex}
        onToggleRegex={() => search.setUseRegex((v) => !v)}
        bookmarksOnly={search.bookmarksOnly}
        onToggleBookmarks={() => search.setBookmarksOnly((v) => !v)}
        allTags={tags.tags}
        activeTags={parseTagQuery(search.query).tags}
        onToggleTag={(name) => search.setQuery(toggleTagInQuery(search.query, name))}
      />

      {search.error && <p style={{ color: "tomato", margin: "0 8px 6px" }}>{search.error}</p>}

      <ul
        className="clip-list"
        onScroll={(e) => {
          const el = e.currentTarget;
          // 末尾 240px 手前まで来たら次ページを継ぎ足す。
          if (el.scrollHeight - el.scrollTop - el.clientHeight < 240) {
            search.loadMore();
          }
        }}
      >
        {search.clips.map((c) => (
          <ClipRow
            key={c.id}
            clip={c}
            copied={copiedId === c.id}
            showBadge={showIcons}
            maxLines={maxLines}
            onPaste={() => handlePaste(c)}
            onInspect={() => handleInspect(c)}
            onRequestMenu={(x, y) => setMenu({ clip: c, x, y })}
          />
        ))}
      </ul>

      {menu && (
        <ClipMenu
          clip={menu.clip}
          x={menu.x}
          y={menu.y}
          onView={() => {
            handleInspect(menu.clip);
            setMenu(null);
          }}
          onCopy={() => handleCopy(menu.clip)}
          onPastePlain={() => handlePastePlain(menu.clip)}
          onEditMeta={() => {
            // メニューを閉じ、同じ位置に説明 / タグの編集パネルを開く。
            setMeta({ clip: menu.clip, x: menu.x, y: menu.y, focus: "note" });
            setMenu(null);
          }}
          onToggleBookmark={() => handleToggleBookmark(menu.clip.id)}
          onDelete={() => handleDelete(menu.clip.id)}
          onClose={() => setMenu(null)}
        />
      )}

      {meta && (
        <ClipMetaPanel
          clip={meta.clip}
          x={meta.x}
          y={meta.y}
          focus={meta.focus}
          allTags={tags.tags}
          onSaved={() => {
            // 保存で clips-changed が飛ぶので一覧は自動更新されるが、
            // 直後の表示がずれないようここでも引き直す。
            search.refresh();
            tags.refresh();
          }}
          onError={search.setError}
          onClose={() => setMeta(null)}
        />
      )}

      {previewSrc && <PreviewModal src={previewSrc} onClose={() => setPreviewSrc(null)} />}

      {source && (
        <SourceModal
          text={source.text}
          html={source.html}
          onClose={() => setSource(null)}
          onPaste={(content) => {
            // 編集後の文字列をそのまま貼り付ける。元と違えば監視側が新規履歴として保存する。
            pasteText(content).catch((e) => search.setError(String(e)));
          }}
        />
      )}

      {onboarding && <Onboarding onFinish={() => setOnboarding(false)} />}
    </main>
  );
}

export default App;
