// version.ts — セマンティックバージョンの簡易比較（更新通知で latest > current を判定）。

// "v1.2.3" / "1.2.3" を数値配列にする。欠けや非数値は 0 として扱う。
function parseVersion(v: string): number[] {
  return v
    .trim()
    .replace(/^v/i, "")
    .split(".")
    .map((part) => {
      const n = parseInt(part, 10);
      return Number.isFinite(n) ? n : 0;
    });
}

// latest が current より新しければ true。プレフィックス "v" の有無は無視する。
export function isNewerVersion(latest: string, current: string): boolean {
  const a = parseVersion(latest);
  const b = parseVersion(current);
  const len = Math.max(a.length, b.length);
  for (let i = 0; i < len; i++) {
    const diff = (a[i] ?? 0) - (b[i] ?? 0);
    if (diff !== 0) return diff > 0;
  }
  return false;
}
