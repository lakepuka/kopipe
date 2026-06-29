import { useState } from "react";

import { useT } from "../i18n";

type View = "plain" | "html" | "web";

// SourceModal — クリップの中身を表示し、その場で編集して貼り付けられるビューワー。
// プレーン / HTML(ソース) は編集可能なテキストエリア、Web は描画プレビュー（編集中の
// HTML を反映）。Web は sandbox 付き iframe でスクリプトを一切実行しない（無害化）。
// 「貼り付け」は表示中の文字列をそのまま元アプリへ貼り付ける。貼り付けた内容は
// クリップボード監視が拾うため、元と違えば新しい履歴として自動保存される。
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

  // 現在のビューで編集対象になっている文字列（Web は HTML ソースを対象にする）。
  const draft = view === "plain" ? plainDraft : htmlDraft;
  const setDraft = view === "plain" ? setPlainDraft : setHtmlDraft;

  const tab = (key: View, label: string) => (
    <button
      type="button"
      className={`source-tab${view === key ? " active" : ""}`}
      onClick={() => setView(key)}
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
            className="source-modal-body"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            spellCheck={false}
            // biome-ignore lint/a11y/noAutofocus: モーダルを開いたら即編集できるようにする。
            autoFocus
          />
        )}
        <div className="source-modal-footer">
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
