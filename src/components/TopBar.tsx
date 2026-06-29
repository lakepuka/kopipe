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
}: {
  query: string;
  onQueryChange: (v: string) => void;
  useRegex: boolean;
  onToggleRegex: () => void;
  bookmarksOnly: boolean;
  onToggleBookmarks: () => void;
}) {
  const t = useT();
  const win = getCurrentWindow();
  return (
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
          className="search"
          value={query}
          onChange={(e) => onQueryChange(e.currentTarget.value)}
          placeholder={useRegex ? t("search_placeholder_regex") : t("search_placeholder")}
        />
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
  );
}
