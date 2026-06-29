import { invoke } from "@tauri-apps/api/core";

// Rust 側 DEFAULT_SHORTCUT と揃える。"double:Shift" = Shift の2回連続。
export const DEFAULT_SHORTCUT = "double:Shift";

// 修飾キー単体の code（これらだけ押されても確定しない）。
const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "ShiftLeft",
  "ShiftRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
]);

type KeyCombo = {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  code: string;
};

// キー入力から、Tauri/global-hotkey 形式のアクセラレータ文字列を組み立てる。
// 修飾キー単体や、修飾キー無しの場合は null（＝未確定）を返す。
export function buildAccelerator(e: KeyCombo): string | null {
  if (MODIFIER_CODES.has(e.code)) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Control");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");
  if (parts.length === 0) return null; // 修飾キーが必須
  parts.push(e.code);
  return parts.join("+");
}

// トリガー値を人が読みやすい表記にする（2回押し or コンボ）。
export function describeShortcut(value: string): string {
  if (value === "double:Shift") return "Shift 2回";
  if (value === "double:Control") return "Ctrl 2回";
  return formatAccelerator(value);
}

// アクセラレータを人が読みやすい表記にする（例: Control+Shift+KeyV → Ctrl + Shift + V）。
export function formatAccelerator(accel: string): string {
  return accel
    .split("+")
    .map((p) => {
      if (p === "Control") return "Ctrl";
      if (p === "Super") return "Win";
      if (p.startsWith("Key")) return p.slice(3);
      if (p.startsWith("Digit")) return p.slice(5);
      return p;
    })
    .join(" + ");
}

// 保存済みショートカットを取得（未設定は既定）。
export async function loadShortcut(): Promise<string> {
  try {
    const s = await invoke<Record<string, string>>("get_settings");
    return s.shortcut || DEFAULT_SHORTCUT;
  } catch {
    return DEFAULT_SHORTCUT;
  }
}
