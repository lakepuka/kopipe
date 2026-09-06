# アーキテクチャ制約

## レイヤと責務

```
React UI (src/**)
   ↓ 呼ぶのは src/services/api.ts だけ
api.ts  ── invoke("cmd", args) ──▶  #[tauri::command] (src-tauri/src/**)
                                       ↓
                                   純関数（clip_search / settings / image_storage）
                                       ↓
                                   SQLite (rusqlite, AppState の Mutex<Connection>)
```

守ること:

- **UI から `invoke` を直接呼ばない。** 新しいコマンドを足したら必ず
  [src/services/api.ts](../../src/services/api.ts) に型付きラッパを追加し、UI はそれを使う。
  コマンド名の文字列がコンポーネントに散らばると、リネーム時に壊れても型で気づけない。
- **`#[tauri::command]` は薄いラッパにする。** ロジックは `&Connection` などを受け取る
  純関数に切り出し、そちらを単体テストする（[settings.rs](../../src-tauri/src/settings.rs) /
  [clip_search.rs](../../src-tauri/src/clip_search.rs) がその形）。
- **OS 依存は `src-tauri/src/system/` に閉じ込める。** `clips.rs` や `db.rs` から
  Win32 API を直接叩かない。`system/mod.rs` は `#![cfg(desktop)]`。
- Win32 の低レベル（フォーカス奪取、Ctrl+V 送出、修飾キー 2 回のフック）は
  [system/win32.rs](../../src-tauri/src/system/win32.rs) のみ。ウィンドウ表示制御は
  `system/window.rs`、トリガー登録は `system/shortcut.rs`。

## 新しいコマンドを足す手順

1. `src-tauri/src/**` に `#[tauri::command]` を書く（戻りのエラーは `Result<_, String>`）。
2. [lib.rs](../../src-tauri/src/lib.rs) の `invoke_handler![...]` に登録する（**忘れやすい**）。
3. `src/services/api.ts` にラッパを追加。Rust の `snake_case` 引数は JS 側では
   **camelCase** で渡す（Tauri が変換する）。
4. Tauri プラグインの機能を使うなら
   [capabilities/default.json](../../src-tauri/capabilities/default.json) に permission を追加する。
   権限は必要最小限（`security.md` 参照）。

## イベント（Rust → フロント）

イベントは 3 つだけ。新設する前に、この 3 つで表現できないか考える。

- `clips-changed` — 履歴が変わった（自動記録・削除・ブックマーク・説明/タグ・画像整理）。
  `useClips` と `useTags` が購読して現在の検索条件で引き直す。
- `settings-changed` — 設定が変わった。設定画面・主画面・`I18nProvider` が追従する。
- `main-shown` — main が隠れた状態から表示された（起動・トレイ・ショートカット・2 回押し）。
  `system/window.rs` の `mark_main_shown` の 1 点だけが emit する。設定ウィンドウの表示では
  鳴らさない。いまの購読者は「呼び出すたびに検索条件をリセット」だけ。

emit しっぱなしで良い（`let _ = app.emit(...)`）が、**状態を変えたら必ず emit する**。
emit を忘れると「保存したのに画面が古いまま」になる。

## 設定（key/value）

- 実体は SQLite の `settings` テーブル（`settings.rs`）。
- キーは [src/lib/prefs.ts](../../src/lib/prefs.ts) の `SETTING_KEYS` に**必ず**登録し、
  Rust 側で使う文字列と一致させる。UI に生キー文字列を書かない。
- 値は文字列。パースは `prefs.ts` の `parseXxx` に集約し、**未設定時の既定値をそこで決める**
  （例: `update_check` は未設定=有効のオプトアウト式）。
- 複数項目を読むときは `getSettings()` / `loadDisplay()` で 1 回の IPC にまとめる。

## DB とマイグレーション

- スキーマ変更は [db.rs](../../src-tauri/src/db.rs) の `open_state` 内で行う。
  既存ユーザーの DB を壊さないこと。列追加は `ensure_column`、改名は
  `rename_column_if_present` を使う（起動時に冪等に走る）。
- 破壊的な変更（列削除・型変更）は避ける。どうしても必要なら移行コードとテストをセットで。
- `AppState::lock()` の `MutexGuard` を握ったまま重い処理や別ロック取得をしない（デッドロック）。

## i18n

- 辞書は [src/i18n.tsx](../../src/i18n.tsx)。**`ja` がマスター**で、`TKey = keyof typeof dict.ja`。
- 文言を足すときは `ja` と `en` の両方に足す（`en` 欠落時は `ja` にフォールバックするため、
  型エラーにならず日本語が英語 UI に漏れる）。
- UI に文字列リテラルを直書きしない。`useT()` の `t("key")` を使う。

## ウィンドウの挙動（壊しやすい）

- `main` / `settings` は閉じても破棄せず `hide()`（再生成のデッドロックを避ける）。
- 外部アプリへフォーカスが移ったら畳む。ただし自分の別ウィンドウ・ダイアログ・
  表示直後の一過性 blur は除外する（`foreground_is_ours` / `blur_suppressed`）。
- ピン留め中は main を残し、次の貼り付け先として直前のフォアグラウンドを記録する。
- 2 重起動は single-instance プラグインで既存インスタンスを表示するだけにする。
  ログイン自動起動のときだけウィンドウを出さない（`autostart::launched_by_autostart`）。

これらは実機の挙動を見ないと壊れたことに気づけない。触ったら testing.md の手順で起動確認する。
