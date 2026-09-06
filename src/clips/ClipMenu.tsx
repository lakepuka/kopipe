import type { Clip } from "../services/api";
import { useT } from "../i18n";

// カーソル位置に出すコンテキストメニュー。画面外にはみ出さないよう簡易クランプする。
export function ClipMenu({
  clip,
  x,
  y,
  onView,
  onCopy,
  onPastePlain,
  onEditMeta,
  onToggleBookmark,
  onDelete,
  onClose,
}: {
  clip: Clip;
  x: number;
  y: number;
  onView: () => void;
  onCopy: () => void;
  onPastePlain: () => void;
  onEditMeta: () => void;
  onToggleBookmark: () => void;
  onDelete: () => void;
  onClose: () => void;
}) {
  const t = useT();
  const W = 200;
  const H = 260;
  const left = Math.max(4, Math.min(x, window.innerWidth - W - 4));
  const top = Math.max(4, Math.min(y, window.innerHeight - H - 4));

  return (
    <>
      {/* メニュー外クリック（右クリック含む）で閉じる透明オーバーレイ。 */}
      <button
        type="button"
        tabIndex={-1}
        aria-label="close menu"
        onClick={onClose}
        onContextMenu={(e) => {
          e.preventDefault();
          onClose();
        }}
        style={{
          position: "fixed",
          inset: 0,
          zIndex: 40,
          border: "none",
          padding: 0,
          background: "transparent",
        }}
      />
      <div
        className="context-menu"
        style={{ position: "fixed", left, top, zIndex: 41, minWidth: 160 }}
      >
        {clip.kind !== "files" && (
          <button type="button" onClick={onView}>
            {t("view")}
          </button>
        )}
        {clip.kind !== "image" && (
          <button type="button" onClick={onCopy}>
            {t("copy")}
          </button>
        )}
        {clip.kind !== "image" && (
          <button type="button" onClick={onPastePlain}>
            {t("paste_plain")}
          </button>
        )}
        {/* 区切りで「中身を扱う操作」「メタ情報」「行そのもの」に分ける。
            項目が増えても読み取りが重くならないように。 */}
        <div className="menu-sep" />
        <button type="button" onClick={onEditMeta}>
          {t("edit_note_tags")}
        </button>
        <div className="menu-sep" />
        <button type="button" onClick={onToggleBookmark}>
          {clip.bookmark ? t("bookmark_remove") : t("bookmark_add")}
        </button>
        <button type="button" onClick={onDelete} style={{ color: "tomato" }}>
          {t("delete")}
        </button>
      </div>
    </>
  );
}
