import { getCurrentWindow } from "@tauri-apps/api/window";

import { useT } from "../i18n";
import { openSettings } from "../services/api";

// 共通のカスタムタイトルバー。ドラッグ移動＋（任意で）設定/最小化＋閉じる(トレイへ)。
export function TitleBar({
  title = "kopipe",
  showSettings = false,
  showMinimize = false,
}: {
  title?: string;
  showSettings?: boolean;
  showMinimize?: boolean;
}) {
  const t = useT();
  const win = getCurrentWindow();
  return (
    <div className="titlebar" data-tauri-drag-region>
      <span className="titlebar-title" data-tauri-drag-region>
        {title}
      </span>
      {showSettings && (
        <button
          type="button"
          className="icon-btn"
          title={t("settings")}
          onClick={() => openSettings()}
        >
          ⚙
        </button>
      )}
      {showMinimize && (
        <button
          type="button"
          className="icon-btn"
          title={t("minimize")}
          onClick={() => win.minimize()}
        >
          —
        </button>
      )}
      <button type="button" className="icon-btn" title={t("close")} onClick={() => win.hide()}>
        ✕
      </button>
    </div>
  );
}
