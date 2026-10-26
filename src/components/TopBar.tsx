import { useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { useT } from "../i18n";
import { openSettings } from "../services/api";

// メインウィンドウ上部の一体型バー。
// ドラッグ移動・ブランド・検索・正規表現/ブックマークトグル・設定/閉じる。
export function TopBar({
  query,
  onQueryChange,
  useRegex,
  onToggleRegex,
  bookmarksOnly,
  onToggleBookmarks,
  allTags,
  activeTags,
  onToggleTag,
}: {
  query: string;
  onQueryChange: (v: string) => void;
  useRegex: boolean;
  onToggleRegex: () => void;
  bookmarksOnly: boolean;
  onToggleBookmarks: () => void;
  allTags: string[];
  activeTags: string[];
  onToggleTag: (name: string) => void;
}) {
  const t = useT();
  // タグバーの開閉。絞り込み中は「何で絞られているか」を隠さないため常に開く。
  const [tagBarOpen, setTagBarOpen] = useState(false);
  const showTagBar = allTags.length > 0 && (tagBarOpen || activeTags.length > 0);
  const isActive = (name: string) => activeTags.some((n) => n.toLowerCase() === name.toLowerCase());
  const win = getCurrentWindow();
  const inputRef = useRef<HTMLInputElement>(null);
  return (
    <>
      <div className="topbar" data-tauri-drag-region>
        <span className="brand" data-tauri-drag-region>
          kopipe
        </span>
        {/* brand と検索の間の空白。ここはドラッグ専用で確実に掴める。 */}
        <div className="drag-spacer" data-tauri-drag-region />
        <div className="search-wrap">
          <svg
            className="search-icon"
            viewBox="0 0 16 16"
            width="14"
            height="14"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
            aria-hidden
          >
            <title>{t("search_placeholder")}</title>
            <circle cx="7" cy="7" r="4.5" />
            <line x1="10.5" y1="10.5" x2="14" y2="14" />
          </svg>
          <input
            ref={inputRef}
            className="search"
            value={query}
            onChange={(e) => onQueryChange(e.currentTarget.value)}
            placeholder={useRegex ? t("search_placeholder_regex") : t("search_placeholder")}
          />
          {/* 入力があるときだけ表示するクリアボタン。入力のフォーカスは保つ。 */}
          {query && (
            <button
              type="button"
              className="affix-btn"
              onClick={() => {
                onQueryChange("");
                inputRef.current?.focus();
              }}
              title={t("clear_search")}
              aria-label={t("clear_search")}
            >
              <svg
                viewBox="0 0 16 16"
                width="12"
                height="12"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.6"
                strokeLinecap="round"
                aria-hidden
              >
                <title>{t("clear_search")}</title>
                <line x1="4" y1="4" x2="12" y2="12" />
                <line x1="12" y1="4" x2="4" y2="12" />
              </svg>
            </button>
          )}
          {/* 検索条件のトグルは入力欄の中（末尾）に収める。 */}
          <button
            type="button"
            className={`affix-btn${useRegex ? " toggled" : ""}`}
            onClick={onToggleRegex}
            title={useRegex ? t("regex_on") : t("regex_off")}
            aria-pressed={useRegex}
            style={{ fontFamily: "monospace" }}
          >
            .*
          </button>
          {/* タグが 1 つも無いうちはボタン自体を出さない（使っていない人の画面を変えない）。 */}
          {allTags.length > 0 && (
            <button
              type="button"
              className={`affix-btn${activeTags.length > 0 ? " toggled" : ""}`}
              onClick={() => setTagBarOpen((v) => !v)}
              title={activeTags.length > 0 ? t("tag_filter_on") : t("tag_filter_off")}
              aria-pressed={showTagBar}
            >
              <svg
                viewBox="0 0 16 16"
                width="13"
                height="13"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinejoin="round"
                aria-hidden
              >
                <title>{activeTags.length > 0 ? t("tag_filter_on") : t("tag_filter_off")}</title>
                <path d="M2.5 2.5h5l6 6-5 5-6-6z" />
                <circle cx="5" cy="5" r="1" fill="currentColor" stroke="none" />
              </svg>
            </button>
          )}
          <button
            type="button"
            className={`affix-btn bookmark${bookmarksOnly ? " toggled" : ""}`}
            onClick={onToggleBookmarks}
            title={bookmarksOnly ? t("bookmark_on") : t("bookmark_off")}
            aria-pressed={bookmarksOnly}
          >
            <svg
              viewBox="0 0 16 16"
              width="13"
              height="13"
              fill={bookmarksOnly ? "currentColor" : "none"}
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinejoin="round"
              aria-hidden
            >
              <title>{bookmarksOnly ? t("bookmark_on") : t("bookmark_off")}</title>
              <path d="M4 2.5h8v11l-4-3-4 3z" />
            </svg>
          </button>
        </div>
        <button
          type="button"
          className="icon-btn"
          onClick={() => openSettings()}
          title={t("settings")}
        >
          ⚙
        </button>
        <button type="button" className="icon-btn" onClick={() => win.hide()} title={t("close")}>
          ✕
        </button>
      </div>

      {/* 絞り込みバーは 2 行目に窓幅いっぱいで左から並べ、収まらない分だけ折り返す。
        押すと検索文字列の tag: を書き換えるだけ（状態を 2 つ持たない）。 */}
      {showTagBar && (
        <div className="tag-bar">
          {allTags.map((name) => (
            <button
              key={name}
              type="button"
              className={`tag-chip${isActive(name) ? " on" : ""}`}
              aria-pressed={isActive(name)}
              onClick={() => onToggleTag(name)}
            >
              {name}
            </button>
          ))}
        </div>
      )}
    </>
  );
}
