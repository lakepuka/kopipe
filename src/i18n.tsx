import { createContext, type ReactNode, useContext, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Lang = "ja" | "en";
export const DEFAULT_LANG: Lang = "en";

// 翻訳辞書。ja をマスターとし、キーは ja から導出する。
const dict = {
  ja: {
    settings: "設定",
    minimize: "最小化",
    close: "閉じる",
    search_placeholder: "検索...",
    search_placeholder_regex: "正規表現で検索...",
    regex_on: "正規表現モード ON",
    regex_off: "正規表現モード OFF",
    bookmark_on: "ブックマークのみ表示中",
    bookmark_off: "ブックマークのみ表示",
    tip_paste: "クリックで元アプリに貼り付け",
    view_text: "全文を表示",
    view_html: "HTML ソースを表示",
    open_in_explorer: "エクスプローラーで開く",
    plain: "プレーン",
    expand: "全文を表示",
    collapse: "折りたたむ",
    image_loading: "🖼 画像…",
    preview: "プレビュー",
    detail: "詳細",
    view: "表示",
    copy: "コピー",
    paste: "貼り付け",
    paste_plain: "プレーン文字として貼り付け",
    bookmark_add: "ブックマークに追加",
    bookmark_remove: "ブックマーク解除",
    delete: "削除",
    copied: "コピー！",
    appearance: "表示",
    show_row_icons: "行の先頭アイコンを表示",
    pin_window: "最前面に固定（ピン留め）",
    max_lines: "最大表示行数",
    lines_unit: "行",
    all_lines: "全行",
    startup: "スタートアップ",
    onboard_lang_title: "言語を選択",
    next: "次へ",
    later: "あとで",
    enable: "有効にする",
    help_title: "kopipe へようこそ",
    help_body:
      "kopipe はコピー＆ペーストの履歴ツールです。コピーした内容が自動でたまり、すぐに貼り付けられます。\n\n初期設定では Shift キーを2回押すと、いつでも呼び出せます。",
    help_ok: "はじめる",
    launch_at_startup: "PC ログイン時から履歴の記録を開始（トレイに常駐）",
    autostart_prompt_title: "PC ログイン時に常駐起動",
    autostart_prompt_message:
      "PC のログイン時に kopipe をトレイへ常駐起動して、\nクリップボード履歴の記録を開始します。\nウィンドウは表示されません。\n\n有効にしますか？（後で設定からも変更できます）",
    theme: "テーマ",
    theme_kopipe: "デフォルト（kopipe）",
    theme_light: "ライト",
    theme_dark: "ダーク",
    theme_mint: "ミント",
    theme_grape: "グレープ",
    theme_honey: "ハニー",
    theme_sky: "スカイ",
    theme_system: "システムに合わせる",
    trigger: "呼び出し方法",
    trigger_double_shift: "Shift 2回",
    trigger_double_ctrl: "Ctrl 2回",
    trigger_combo: "キーの組み合わせ",
    recording: "キーを押す…（Escで取消）",
    unset: "未設定",
    image_dir: "画像の保存先",
    default_paren: "(既定)",
    capacity: "容量:",
    limit: "上限",
    limit_hint: "超過すると古い画像から自動削除（ブックマークは対象外）",
    change_folder: "フォルダを変更…",
    data: "データ",
    clear_text: "履歴を消す",
    confirm_clear_text: "テキスト履歴を消しますか？",
    clear_files: "パス履歴を消す",
    confirm_clear_files: "パス（ファイル/フォルダ）履歴を消しますか？",
    clear_image: "画像履歴を消す",
    confirm_clear_image: "画像履歴を消しますか？",
    do_clear: "消す",
    cancel: "取消",
    reset: "初期設定に戻す",
    confirm_reset: "設定を初期化しますか？",
    do_reset: "戻す",
    language: "言語",
    updates: "更新",
    check_updates: "起動時に更新を確認する",
    update_hint: "GitHub で最新版を確認します（kopipe で唯一の外部通信）。",
    update_available: "新しいバージョンが利用できます：",
    update_get: "入手",
  },
  en: {
    settings: "Settings",
    minimize: "Minimize",
    close: "Close",
    search_placeholder: "Search...",
    search_placeholder_regex: "Search by regex...",
    regex_on: "Regex mode ON",
    regex_off: "Regex mode OFF",
    bookmark_on: "Showing bookmarks only",
    bookmark_off: "Show bookmarks only",
    tip_paste: "Click to paste into the previous app",
    view_text: "Show full text",
    view_html: "Show HTML source",
    open_in_explorer: "Open in Explorer",
    plain: "Plain",
    expand: "Show full text",
    collapse: "Collapse",
    image_loading: "🖼 image…",
    preview: "Preview",
    detail: "Details",
    view: "View",
    copy: "Copy",
    paste: "Paste",
    paste_plain: "Paste as plain text",
    bookmark_add: "Add bookmark",
    bookmark_remove: "Remove bookmark",
    delete: "Delete",
    copied: "Copied!",
    appearance: "Display",
    show_row_icons: "Show leading row icons",
    pin_window: "Keep on top (pin)",
    max_lines: "Max lines per item",
    lines_unit: "lines",
    all_lines: "All",
    startup: "Startup",
    onboard_lang_title: "Choose your language",
    next: "Next",
    later: "Later",
    enable: "Enable",
    help_title: "Welcome to kopipe",
    help_body:
      "kopipe is a copy & paste history tool. Everything you copy is saved automatically and ready to paste.\n\nBy default, double-tap Shift to open it anytime.",
    help_ok: "Get started",
    launch_at_startup: "Record history from PC login (runs in tray)",
    autostart_prompt_title: "Run at PC login",
    autostart_prompt_message:
      "Start kopipe in the tray at PC login so it records\nyour clipboard history. No window is shown.\n\nEnable this? (You can change it later in settings.)",
    theme: "Theme",
    theme_kopipe: "Default (kopipe)",
    theme_light: "Light",
    theme_dark: "Dark",
    theme_mint: "Mint",
    theme_grape: "Grape",
    theme_honey: "Honey",
    theme_sky: "Sky",
    theme_system: "Match system",
    trigger: "Activation",
    trigger_double_shift: "Double Shift",
    trigger_double_ctrl: "Double Ctrl",
    trigger_combo: "Key combo",
    recording: "Press keys… (Esc to cancel)",
    unset: "Not set",
    image_dir: "Image folder",
    default_paren: "(default)",
    capacity: "Size:",
    limit: "Limit",
    limit_hint: "Oldest images are auto-deleted when exceeded (bookmarks kept)",
    change_folder: "Change folder…",
    data: "Data",
    clear_text: "Clear text history",
    confirm_clear_text: "Clear text history?",
    clear_files: "Clear path history",
    confirm_clear_files: "Clear path (file/folder) history?",
    clear_image: "Clear image history",
    confirm_clear_image: "Clear image history?",
    do_clear: "Clear",
    cancel: "Cancel",
    reset: "Reset to defaults",
    confirm_reset: "Reset settings?",
    do_reset: "Reset",
    language: "Language",
    updates: "Updates",
    check_updates: "Check for updates on launch",
    update_hint: "Checks GitHub for the latest version (kopipe's only network request).",
    update_available: "A new version is available:",
    update_get: "Get it",
  },
} as const;

export type TKey = keyof typeof dict.ja;
export type TFn = (key: TKey) => string;

export function parseLang(value: string | undefined | null): Lang {
  return value === "ja" ? "ja" : "en";
}

function makeT(lang: Lang): TFn {
  return (key) => dict[lang][key] ?? dict.ja[key];
}

export async function loadLang(): Promise<Lang> {
  try {
    const s = await invoke<Record<string, string>>("get_settings");
    return parseLang(s.lang);
  } catch {
    return DEFAULT_LANG;
  }
}

// デフォルトは ja（Provider 無しのテスト描画でも ja 文言になる）。
const LangContext = createContext<TFn>(makeT(DEFAULT_LANG));

export function useT(): TFn {
  return useContext(LangContext);
}

// 言語を読み込み、設定変更（settings-changed）に追従して翻訳関数を供給する。
export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLang] = useState<Lang>(DEFAULT_LANG);
  useEffect(() => {
    const apply = () => loadLang().then(setLang);
    apply();
    const unlisten = listen("settings-changed", apply);
    return () => {
      unlisten.then((f) => f());
    };
  }, []);
  return <LangContext.Provider value={makeT(lang)}>{children}</LangContext.Provider>;
}
