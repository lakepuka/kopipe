import type { Clip } from "../services/api";
import { ClipImage } from "./ClipImage";
import { ClipText } from "./ClipText";
import { useT } from "../i18n";

// 行に並べるタグの上限。これを超えた分は「+n」にまとめる（右側を膨らませない）。
const MAX_ROW_TAGS = 2;

// 履歴1行。ブックマークは右上角の青いコーナーリボンで表現、操作はホバーで出る ⋮（表示専用）。
// 先頭の種別バッジはクリックで中身を確認できる（テキスト全文/HTML ソース/エクスプローラー/画像拡大）。
// メニュー本体は App 側で座標指定して描画するため、ここでは開く要求だけ出す。
export function ClipRow({
  clip,
  copied,
  showBadge,
  maxLines,
  onPaste,
  onInspect,
  onRequestMenu,
}: {
  clip: Clip;
  copied: boolean;
  showBadge: boolean;
  maxLines: number;
  onPaste: () => void;
  onInspect: () => void;
  onRequestMenu: (x: number, y: number) => void;
}) {
  const t = useT();

  // 種別ごとの先頭バッジ（クリックで中身を確認）。
  const badge =
    clip.kind === "image"
      ? { label: "IMG", title: t("preview") }
      : clip.kind === "files"
        ? { label: "DIR", title: t("open_in_explorer") }
        : clip.html
          ? { label: "</>", title: t("view_html") }
          : { label: "TXT", title: t("view_text") };

  const content =
    clip.kind === "image" ? (
      <ClipImage id={clip.id} onClick={onPaste} />
    ) : (
      <ClipText content={clip.content} onPaste={onPaste} maxLines={maxLines} />
    );

  return (
    <li
      className={`clip-row${clip.bookmark ? " bookmark" : ""}`}
      // 行ごとに 1 つだけタブ停止し、Enter でペースト（行内のボタンはタブ順から外す）。
      // biome-ignore lint/a11y/noNoninteractiveTabindex: The row is the single keyboard stop; nested action buttons are removed from tab order.
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          onPaste();
        }
      }}
      onContextMenu={(e) => {
        // 右クリックした位置にメニューを出す。
        e.preventDefault();
        onRequestMenu(e.clientX, e.clientY);
      }}
    >
      <div className="clip-body">
        {showBadge ? (
          <div style={{ display: "flex", gap: 6, alignItems: "flex-start" }}>
            <button
              type="button"
              className="row-badge"
              title={badge.title}
              tabIndex={-1}
              onClick={(e) => {
                e.stopPropagation();
                onInspect();
              }}
            >
              {badge.label}
            </button>
            <div style={{ flex: 1, minWidth: 0 }}>{content}</div>
          </div>
        ) : (
          content
        )}
      </div>

      {/* 説明とタグ。中身があるときだけ描画し、無い行は幅を 1px も取らない。
          ラベルは付けず形で区別する＝丸い pill がタグ、薄い文字が説明。
          本文の続きに見えないよう、CSS 側で本文との溝だけは必ず空ける。 */}
      {(clip.tags.length > 0 || clip.note) && (
        <div className="clip-meta">
          {clip.tags.slice(0, MAX_ROW_TAGS).map((name) => (
            <span key={name} className="clip-tag">
              {name}
            </span>
          ))}
          {clip.tags.length > MAX_ROW_TAGS && (
            <span className="clip-tag more" title={clip.tags.join(", ")}>
              +{clip.tags.length - MAX_ROW_TAGS}
            </span>
          )}
          {clip.note && (
            <span className="clip-note" title={`${t("note")}: ${clip.note}`}>
              {clip.note}
            </span>
          )}
        </div>
      )}

      <div className="clip-actions">
        {copied && <span className="clip-copied">{t("copied")}</span>}

        <button
          type="button"
          className="icon-btn"
          title={t("detail")}
          aria-haspopup="true"
          tabIndex={-1}
          style={{ fontSize: 16 }}
          onClick={(e) => {
            // ⋮ ボタンの直下あたりにメニューを出す。
            const r = e.currentTarget.getBoundingClientRect();
            onRequestMenu(r.right, r.bottom);
          }}
        >
          ⋮
        </button>
      </div>
    </li>
  );
}
