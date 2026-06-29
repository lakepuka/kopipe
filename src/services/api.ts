// api.ts — Rust コマンド（invoke）の型付きラッパ。UI から invoke 文字列を散らさない。
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { openUrl } from "@tauri-apps/plugin-opener";

// GitHub リポジトリ（更新通知の参照先）。
const REPO = "lakepuka/kopipe";

// Rust 側の Clip struct と対応する型。
export type Clip = {
  id: number;
  content: string;
  created_at: number; // Unix 秒
  bookmark: boolean;
  kind: "text" | "image" | "files";
  image_path: string | null;
  html: string | null; // リッチ形式があれば HTML 断片
};

// 注: Rust の snake_case 引数は JS では camelCase で渡す（Tauri が自動変換）。
export function searchClips(
  query: string,
  bookmarksOnly: boolean,
  useRegex: boolean,
  limit = 100,
  offset = 0,
): Promise<Clip[]> {
  return invoke<Clip[]>("search_clips", { query, limit, offset, bookmarksOnly, useRegex });
}

export const toggleBookmark = (id: number) => invoke("toggle_bookmark", { id });
export const deleteClip = (id: number) => invoke("delete_clip", { id });
export const imageDataUrl = (id: number) => invoke<string>("image_data_url", { id });
export const openSettings = () => invoke("open_settings");
export const imageStorageBytes = () => invoke<number>("image_storage_bytes");
export const imageDirPath = () => invoke<string>("image_dir_path");
export const appVersion = () => invoke<string>("app_version");
export const setShortcut = (accelerator: string) => invoke("set_shortcut", { accelerator });
export const setImageLimit = (mb: number) => invoke("set_image_limit", { mb });
export const clearClips = (kind: "text" | "image" | "files") => invoke("clear_clips", { kind });
export const resetAppSettings = () => invoke("reset_settings");
export const setPinned = (pinned: boolean) => invoke("set_pinned", { pinned });

// files クリップの content は改行区切りのパス。
export const filePaths = (c: Clip) => c.content.split("\n").filter(Boolean);

// エクスプローラーで開く（ディレクトリは開き、ファイルは選択表示）。
export const revealPath = (path: string) => invoke("reveal_path", { path });

// 行クリックの貼り付け。種別ごとに呼ぶコマンドが違う。
export function pasteClip(c: Clip): Promise<unknown> {
  if (c.kind === "image") return invoke("paste_image", { id: c.id });
  if (c.kind === "files") return invoke("paste_files", { paths: filePaths(c) });
  return invoke("paste_clip", { content: c.content, html: c.html });
}

// メニューの「コピー」。files は実ファイル(CF_HDROP)、HTML 付きは両形式、それ以外はテキスト。
export function copyClip(c: Clip): Promise<unknown> {
  if (c.kind === "files") return invoke("copy_files", { paths: filePaths(c) });
  if (c.html) return invoke("copy_text", { content: c.content, html: c.html });
  return writeText(c.content);
}

// 種別によらず content をプレーン文字として貼り付ける（files はパス文字列、HTML は素のテキスト）。
export const pasteText = (content: string) => invoke("paste_clip", { content, html: null });

// 既定ブラウザで URL を開く（更新通知のリリースページなど）。
export const openExternal = (url: string) => openUrl(url);

// 最新リリースの情報。kopipe で唯一の外部通信（更新通知のため GitHub のみ）。
export type LatestRelease = { tag: string; url: string };

// GitHub の最新リリースを取得する。失敗（未公開・オフライン等）は null を返す。
export async function fetchLatestRelease(): Promise<LatestRelease | null> {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return null;
    const data = (await res.json()) as { tag_name?: string; html_url?: string };
    if (!data.tag_name || !data.html_url) return null;
    return { tag: data.tag_name, url: data.html_url };
  } catch {
    return null;
  }
}
