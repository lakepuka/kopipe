# テスト方針

## 全体像

| 対象 | ツール | 置き場所 |
| --- | --- | --- |
| フロント（純関数・フック・コンポーネント） | Vitest + Testing Library（jsdom） | 対象と同じ階層の `*.test.ts(x)` |
| Rust（クエリ・設定・記録・容量制限） | `cargo test` | 各 `.rs` 末尾の `#[cfg(test)] mod tests` |

実行:

```bash
pnpm verify                                        # format:check + lint + tsc + vitest
cargo test --manifest-path src-tauri/Cargo.toml    # Rust
```

CI（windows-latest）はこの 2 つ＋ `cargo fmt --check` を実行する。ローカルで先に通す。

## フロント

- テストは実装ファイルの**隣**に置く（`api.ts` ↔ `api.test.ts`）。
- Tauri の API は必ずモックする。jsdom には `window.__TAURI__` が無いので、
  実物を呼ぶとテストが落ちる:

  ```ts
  vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(() => Promise.resolve()) }));
  ```

- `api.ts` のテストで見るのは **「どのコマンドを、どの引数で呼んだか」**
  （`expect(mockInvoke).toHaveBeenCalledWith("paste_image", { id: 2 })`）。
  種別（text / image / files / html 付き）で分岐する関数は全分岐を押さえる。
- ロジックは可能な限り `src/lib/`（format / theme / version / shortcut）へ純関数として出し、
  そこを厚くテストする。コンポーネントのテストは「表示されるか・クリックで正しい関数が呼ばれるか」に留める。
- `I18nProvider` 無しで描画すると既定の `ja` 文言になる。文言でアサートするときは注意。

## Rust

- DB を使うテストは `open_state(Path::new(":memory:"))` でインメモリ DB を開く。
  ファイル DB を作らない・実 DB を触らない。
- テスト対象は `#[tauri::command]` ではなく、その下の純関数（`query_clips`、`record_clip`、
  `get_all` / `set`、`enforce_image_limit`、`dedupe_clips` など）。
  `AppHandle` が要る処理はロジックを分離してからテストする。
- 重複排除・繰り上げ・ブックマーク継承・容量超過削除は退行しやすい。仕様を変えたらテストも更新する。

## 何を必ずテストするか

- 検索（クエリ、正規表現、ブックマーク絞り込み、ページング）
- 記録（空文字スキップ、重複時の繰り上げ、HTML 付きテキスト、画像 hash）
- マイグレーション（旧列名からの改名、列追加後も既存行が読める）
- 設定のパースと既定値（未設定時にどちらへ倒れるか）
- ショートカット文字列のパース／表示

## ローカル起動の隔離（必読）

このマシンには kopipe が**常用アプリとして導入され、ログイン時に常駐している**
（`%LOCALAPPDATA%\kopipe\kopipe.exe`）。ローカルビルドは `productName` と
`identifier` が同じため、そのまま起動すると次を共有／破壊する:

- 履歴 DB `%APPDATA%\io.github.lakepuka.kopipe\kopipe.db`
- 自動起動レジストリ `HKCU:\...\CurrentVersion\Run\kopipe`
  （起動時に `enable()` が現在の exe パスで上書きする＝ユーザーの自動起動が dev ビルドを指す）
- グローバルホットキー／低レベルキーボードフック、single-instance ミューテックス

**手順:**

1. `src-tauri/tauri.conf.json` の `productName` と `identifier` を一時的に `-test` 版に変える
   （例: `kopipe-test` / `io.github.lakepuka.kopipe-test`）。
2. ビルドして起動し、確認する。
3. `tauri.conf.json` を**元に戻す**（コミットに混ぜない）。
4. `%APPDATA%\io.github.lakepuka.kopipe-test` を削除する。

ウィンドウの表示状態は GUI ドライバなしでも、PowerShell の `Add-Type` で `EnumWindows` +
`IsWindowVisible` を呼び、対象プロセス ID で絞れば確認できる（ウィンドウタイトルは `kopipe`）。

履歴のダミーデータが要るときは `pnpm run seed [件数]`（既定 1000）。ただし
[seed.rs](../../src-tauri/src/bin/seed.rs) は `IDENTIFIER` を**ハードコードしている**ため、
`tauri.conf.json` を `-test` に変えても **常用アプリの実 DB へ直接 INSERT する**。
テスト用 DB に入れたいときは `seed.rs` の `IDENTIFIER` も同じ `-test` 値へ変え、確認後に戻すこと。
