# AGENTS.md — kopipe

エージェント（Claude Code / Codex / Copilot など）向けの作業指示書。
人間向けの説明は [README.md](README.md) を参照。

## このプロジェクト

kopipe は **Windows 専用・完全ローカル**のクリップボード履歴アプリ。
コピーしたテキスト / ファイル / 画像を自動記録し、Ctrl 2 回で呼び出して貼り付ける。
履歴は `%APPDATA%\io.github.lakepuka.kopipe\kopipe.db`（SQLite）にのみ保存され、
**外部送信は一切しない**（唯一の例外はオプトアウト可能な GitHub 更新チェック）。

スタック: Tauri 2 / React 19 + TypeScript (Vite) / Rust + rusqlite / Biome。

## ディレクトリ

```
src/                 フロントエンド（React）
  services/api.ts    invoke の型付きラッパ（UI から invoke 文字列を散らさない）
  clips/             履歴リストとその UI（行・メニュー・プレビュー）
  settings/          設定画面
  components/        タイトルバー・トップバー・オンボーディング
  lib/               純粋ユーティリティ（format / theme / version / prefs）
  i18n.tsx           翻訳辞書（ja がマスター）
src-tauri/src/       バックエンド（Rust）
  lib.rs             エントリ。プラグイン・ウィンドウイベント・コマンド登録
  clips.rs           履歴の記録・取得・削除（#[tauri::command]）
  clip_search.rs     検索クエリ（純関数、テスト対象）
  db.rs              接続・起動時マイグレーション
  settings.rs        key/value 設定の永続化
  image_storage.rs   画像 PNG の保存・容量制限
  system/            OS 連携（win32 / window / shortcut / clipboard / autostart）
.claude/rules/       詳細ルール（下記リンク）
```

## コマンド

```bash
pnpm install        # 依存インストール
pnpm tauri dev      # 開発起動
pnpm verify         # format:check + lint + tsc + vitest（フロントの必須チェック）
pnpm test           # vitest のみ
pnpm run seed 1000  # 開発用のダミー履歴を投入（ローカル DB に直接書く）
pnpm tauri build    # インストーラ生成 → src-tauri/target/release/bundle/nsis/
```

Rust 側:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
```

## 変更したら必ず通すこと

1. フロントを触った → `pnpm verify`
2. Rust を触った → `cargo fmt --check` と `cargo test`（上記コマンド）
3. CI（[ci.yml](.github/workflows/ci.yml)）は windows-latest でこの両方を実行する。
   ローカルで通ることを確認してから提出する。

`pnpm tauri build` / `cargo run` でローカルビルドを**起動**して検証する場合は、
必ず [.claude/rules/testing.md](.claude/rules/testing.md) の「ローカル起動の隔離」に従うこと。
このマシンでは kopipe が常用アプリとして稼働しており、そのまま起動すると
実データと自動起動設定を壊す。

## 規約

- **コメントは日本語**、識別子・型名・ファイル名は英語。コメントは「何をしているか」より
  **なぜそうしたか**（Win32 の癖、UX 上の判断、既知のバグ回避）を書く。既存コードの密度に合わせる。
- **フォーマットは自動**: TS/TSX は Biome（space 2 / lineWidth 100 / double quote / semicolons /
  trailing comma all / **改行は LF**）、Rust は rustfmt。手で整形しない。
- 改行は `.gitattributes` により全ファイル LF。CRLF を持ち込むと CI のフォーマット検査が落ちる。
- TypeScript は `strict` + `noUnusedLocals` + `noUnusedParameters`。`any` を足さない。
- 依存追加は最小限に。追加する場合は [.claude/rules/security.md](.claude/rules/security.md) を先に読む。

## コミット

- **`git commit` の前に必ず内容を提示して承認を得る**（対象ファイルとメッセージ案）。
  `git add` までは可。
- メッセージは Conventional Commits・英語・1 行（例: `fix: refocus search input after clearing it`）。
  使う type: `feat` / `fix` / `docs` / `style` / `chore` / `ci`。
- `Co-Authored-By: Claude` などの trailer は**付けない**（履歴から除去済み）。
- 作業ブランチは `main`。リリース手順は [.claude/rules/release.md](.claude/rules/release.md)。

## 詳細ルール

- [アーキテクチャ制約](.claude/rules/architecture.md) — レイヤ分割、IPC / イベント契約、設定・i18n の追加手順
- [テスト方針](.claude/rules/testing.md) — 何をどう試験するか、ローカル起動の隔離
- [セキュリティ要件](.claude/rules/security.md) — ローカル完結の不変条件、CSP、権限、SQL、ログ
- [リリース手順](.claude/rules/release.md) — バージョン更新箇所、タグ、リリースノートの型
