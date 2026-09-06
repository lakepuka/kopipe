// tagQuery.ts — 検索文字列に埋め込むタグ絞り込み（`tag:仕事,SQL`）の読み書き。
//
// 絞り込み状態を別に持たず「検索文字列 1 本」を真実にするための純関数。
// タグバーのトグルはここを通して検索文字列を書き換えるだけなので、
// 隠れたフィルタが生まれず、検索欄の ✕ ひとつで全部解除できる。
// Rust 側の clip_search::parse_query と同じ規則にすること。

const PREFIX = "tag:";

const isTagToken = (token: string) => token.toLowerCase().startsWith(PREFIX);

// 大小文字を無視した一致（タグ名は NOCASE で一意）。
const sameTag = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

// 検索文字列を「本文検索の部分」と「絞り込みタグ」に分ける。
export function parseTagQuery(query: string): { text: string; tags: string[] } {
  const tokens = query.split(/\s+/).filter(Boolean);
  const text: string[] = [];
  const tags: string[] = [];
  for (const token of tokens) {
    if (!isTagToken(token)) {
      text.push(token);
      continue;
    }
    for (const raw of token.slice(PREFIX.length).split(",")) {
      const name = raw.trim();
      if (name && !tags.some((t) => sameTag(t, name))) tags.push(name);
    }
  }
  return { text: text.join(" "), tags };
}

// タグの絞り込みを追加／解除した検索文字列を返す。
// tag: トークンは末尾に 1 つだけまとめ、既存の入力はそのままの順で前に残す。
export function toggleTagInQuery(query: string, tag: string): string {
  const { text, tags } = parseTagQuery(query);
  const next = tags.some((t) => sameTag(t, tag))
    ? tags.filter((t) => !sameTag(t, tag))
    : [...tags, tag];
  const parts = [text, next.length > 0 ? `${PREFIX}${next.join(",")}` : ""].filter(Boolean);
  return parts.join(" ");
}
