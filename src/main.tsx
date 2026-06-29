import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Settings from "./settings/Settings";
import { I18nProvider } from "./i18n";
import { applyTheme, DEFAULT_THEME } from "./lib/theme";
import "./styles.css";

// 初期フラッシュ低減のため、まず既定テーマを即適用（後で保存値で上書きされる）。
applyTheme(DEFAULT_THEME);

// 既定の右クリックメニュー（再読込/検証/テキスト選択等）を抑止してネイティブ感を出す。
// テキスト入力欄では貼り付け等のため既定を残す。
document.addEventListener("contextmenu", (e) => {
  const el = e.target as HTMLElement;
  if (!el.closest("input, textarea")) e.preventDefault();
});

// URL のクエリ ?view=settings で画面を出し分ける（Tauri API 非依存で確実）。
const isSettings = new URLSearchParams(window.location.search).get("view") === "settings";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider>{isSettings ? <Settings /> : <App />}</I18nProvider>
  </React.StrictMode>,
);
