// settings.ts — 設定キーの一元管理と、設定値の取得/保存ヘルパ。
// 文字列キーの散らばり（タイポ源）を1箇所に集約し、取得も1回の get_settings で済ませる。
import { invoke } from "@tauri-apps/api/core";

// 設定キー（Rust 側の settings テーブルのキーと一致させること）。
export const SETTING_KEYS = {
  theme: "theme",
  lang: "lang",
  showRowIcons: "show_row_icons",
  maxLines: "max_lines",
  pinned: "pinned",
  imageDir: "image_dir",
  imageLimitMb: "image_limit_mb",
  autostartPrompted: "autostart_prompted",
  updateCheck: "update_check",
} as const;

// 文字列値のパース（既定値つき）。
export const parseShowIcons = (v?: string) => v !== "false"; // 未設定は表示
export const parseMaxLines = (v?: string) => {
  const n = parseInt(v ?? "", 10);
  return Number.isFinite(n) ? n : 1; // 0 は「全行」、未設定は 1
};
export const parsePinned = (v?: string) => v === "true";
export const parseUpdateCheck = (v?: string) => v !== "false"; // 未設定は有効（オプトアウト式）

/// 全設定をまとめて取得（失敗時は空）。複数項目を読むときの IPC を1回にまとめる。
export async function getSettings(): Promise<Record<string, string>> {
  try {
    return await invoke<Record<string, string>>("get_settings");
  } catch {
    return {};
  }
}

/// 主画面の表示系設定（アイコン/行数/ピン）を1回の取得でまとめて読む。
export async function loadDisplay(): Promise<{
  showIcons: boolean;
  maxLines: number;
  pinned: boolean;
}> {
  const s = await getSettings();
  return {
    showIcons: parseShowIcons(s[SETTING_KEYS.showRowIcons]),
    maxLines: parseMaxLines(s[SETTING_KEYS.maxLines]),
    pinned: parsePinned(s[SETTING_KEYS.pinned]),
  };
}

/// 設定を1件保存する。保存すると Rust が "settings-changed" を emit し主画面も追従する。
export async function saveSetting(key: string, value: string): Promise<void> {
  try {
    await invoke("set_setting", { key, value });
  } catch (e) {
    console.error(e);
  }
}
