import { useRef, useState } from "react";

import { useT } from "../i18n";

type View = "plain" | "html" | "web";

// SourceModal — クリップの中身を表示し、その場で編集して貼り付けられるビューワー。
// プレーン / HTML(ソース) は編集可能なテキストエリア、Web は描画プレビュー（編集中の
// HTML を反映）。Web は sandbox 付き iframe でスクリプトを一切実行しない（無害化）。
// 「貼り付け」は表示中の文字列をそのまま元アプリへ貼り付ける。「選択部分のみ貼り付け」は
// テキストエリアで選んだ範囲だけを貼る。どちらも paste_clip 経由なので、貼った内容は
// 元と違えば新しい履歴として保存される（バックエンドが貼り付けと同時に記録する）。
export function SourceModal({
  text,
  html,
  onClose,
  onPaste,
}: {
  text: string;
  html: string | null;
  onClose: () => void;
  onPaste: (content: string) => void;
}) {
  const t = useT();
  const [view, setView] = useState<View>("plain");
  const [plainDraft, setPlainDraft] = useState(text);
  const [htmlDraft, setHtmlDraft] = useState(html ?? "");
  const bodyRef = useRef<HTMLTextAreaElement>(null);
  // 範囲選択があるか。ボタンを押すとテキストエリアは blur するが選択範囲は保持されるので、
  // 貼り付け時に ref から読む。ここで持つのはボタンの有効/無効の切替のためだけ。
  const [hasSelection, setHasSelection] = useState(false);

  // 現在のビューで編集対象になっている文字列（Web は HTML ソースを対象にする）。
  const draft = view === "plain" ? plainDraft : htmlDraft;
  const setDraft = view === "plain" ? setPlainDraft : setHtmlDraft;

  const tab = (key: View, label: string) => (
    <button
      type="button"
      className={`source-tab${view === key ? " active" : ""}`}
      onClick={() => {
        setView(key);
        // タブを替えると表示内容ごと選択が失われる（onSelect も飛ばない）ので畳んでおく。
        setHasSelection(false);
      }}
    >
      {label}
    </button>
  );

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 50,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "rgba(0,0,0,0.7)",
        padding: 24,
      }}
    >
      <button
        type="button"
        tabIndex={-1}
        aria-label={t("close")}
        onClick={onClose}
        style={{
          position: "absolute",
          inset: 0,
          border: "none",
          padding: 0,
          background: "transparent",
        }}
      />
      {/* HTML があるときは高さを固定し、タブ切替で上下に動かないようにする。 */}
      <div className={`source-modal${html ? " tabbed" : ""}`} style={{ position: "relative" }}>
        {html && (
          <div className="source-modal-tabs">
            {tab("plain", t("plain"))}
            {tab("html", "HTML")}
            {tab("web", "Web")}
          </div>
        )}
        {view === "web" && html ? (
          // sandbox（許可なし）でスクリプトを実行させずに、編集中の HTML を描画する。
          <iframe className="source-modal-web" sandbox="" srcDoc={htmlDraft} title="web" />
        ) : (
          <textarea
            ref={bodyRef}
            className="source-modal-body"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onSelect={(e) =>
              setHasSelection(e.currentTarget.selectionStart !== e.currentTarget.selectionEnd)
            }
            spellCheck={false}
            // biome-ignore lint/a11y/noAutofocus: モーダルを開いたら即編集できるようにする。
            autoFocus
          />
        )}
        <div className="source-modal-footer">
          <button
            type="button"
            disabled={!hasSelection}
            onClick={() => {
              const el = bodyRef.current;
              if (!el) return;
              const selected = el.value.slice(el.selectionStart, el.selectionEnd);
              // 空白だけの選択は記録側が捨てるので、貼る意味もない。
              if (!selected.trim()) return;
              onPaste(selected);
              onClose();
            }}
          >
            {t("paste_selection")}
          </button>
          <button
            type="button"
            className="source-paste-btn"
            onClick={() => {
              onPaste(draft);
              onClose();
            }}
          >
            {t("paste")}
          </button>
        </div>
      </div>
    </div>
  );
}
