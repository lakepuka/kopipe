import { invoke } from "@tauri-apps/api/core";

// 選べるテーマは名前付き（色の個別指定はしない）。
export type Theme = "kopipe" | "light" | "dark" | "mint" | "grape" | "honey" | "sky" | "system";

// system 以外は固定の配色を持つ。
type NamedTheme = Exclude<Theme, "system">;

export const DEFAULT_THEME: Theme = "kopipe";

export const THEME_LABELS: { value: Theme; label: string }[] = [
  { value: "kopipe", label: "デフォルト（kopipe）" },
  { value: "light", label: "ライト" },
  { value: "dark", label: "ダーク" },
  { value: "mint", label: "ミント（薄緑）" },
  { value: "grape", label: "グレープ（紫）" },
  { value: "honey", label: "ハニー（黄）" },
  { value: "sky", label: "スカイ（薄水色）" },
  { value: "system", label: "システムに合わせる" },
];

type Palette = { fg: string; bg: string; accent: string };

// 各テーマの配色（前景=文字 / 背景 / アクセント）。
// 指定の無いアクセントは前景色を流用してその配色で統一する。
const PALETTES: Record<NamedTheme, Palette> = {
  // kopipe らしい、ダーク＋ティールの配色。
  kopipe: { fg: "#e8f1ef", bg: "#16201e", accent: "#2dd4bf" },
  light: { fg: "#0f0f0f", bg: "#f6f6f6", accent: "#14b8a6" },
  dark: { fg: "#f6f6f6", bg: "#2f2f2f", accent: "#14b8a6" },
  // 薄緑のやさしい配色。文字は濃い緑でくっきり、アクセントは生き生きした緑。
  mint: { fg: "#13402b", bg: "#e6f4ea", accent: "#2f9e57" },
  // 斬新な紫系。淡いラベンダー地に濃いインディゴ文字＋ビビッドな violet。
  grape: { fg: "#3a1d6e", bg: "#efe9fb", accent: "#7c3aed" },
  // 黄色系。やわらかい黄地に濃い琥珀文字＋アンバーのアクセント。
  honey: { fg: "#4a3a07", bg: "#fdf3d3", accent: "#d97706" },
  // 薄い水色にほんのり暖かみ（黄み）を足した、やわらかい空色。
  sky: { fg: "#34555f", bg: "#e7f1ec", accent: "#2f8fb0" },
};

// 不正値は既定テーマにフォールバックする（純関数：テスト対象）。
export function parseTheme(value: string | undefined | null): Theme {
  return value && value in PALETTES
    ? (value as Theme)
    : value === "system"
      ? "system"
      : DEFAULT_THEME;
}

// テーマ名を配色に解決する。system は OS 設定で light/dark を選ぶ。
export function resolvePalette(theme: Theme): Palette {
  if (theme === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? PALETTES.dark
      : PALETTES.light;
  }
  return PALETTES[theme];
}

// テーマを CSS 変数(--fg/--bg/--accent)に反映する。派生色は CSS の color-mix が担当。
export function applyTheme(theme: Theme) {
  const p = resolvePalette(theme);
  const root = document.documentElement.style;
  root.setProperty("--fg", p.fg);
  root.setProperty("--bg", p.bg);
  root.setProperty("--accent", p.accent);
}

// 保存済みテーマを取得（未設定や不正値は既定）。
export async function loadTheme(): Promise<Theme> {
  try {
    const s = await invoke<Record<string, string>>("get_settings");
    return parseTheme(s.theme);
  } catch {
    return DEFAULT_THEME;
  }
}
