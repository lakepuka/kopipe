# CLAUDE.md

kopipe — Windows 専用・完全ローカルのクリップボード履歴アプリ（Tauri 2 + React + Rust）。

作業指示はツール横断の [AGENTS.md](AGENTS.md) に集約している。**まずそれを読むこと。**

@AGENTS.md

## Claude Code 固有のメモ

- 詳細ルールは `.claude/rules/` にある。着手前に該当するものを読む。
  - [architecture.md](.claude/rules/architecture.md) — レイヤ、IPC / イベント契約、設定・i18n の追加手順
  - [testing.md](.claude/rules/testing.md) — テスト方針と、ローカル起動時の隔離手順（**重要**）
  - [security.md](.claude/rules/security.md) — ローカル完結の不変条件、CSP、権限、SQL
  - [release.md](.claude/rules/release.md) — バージョン更新箇所とリリースノートの型
- `git commit` は**承認を得てから**実行する（`git add` までは可）。
- このマシンの kopipe は常用アプリとして稼働中。ローカルビルドを起動する検証は
  testing.md の隔離手順なしに行わない。
- 回答・説明は日本語で。コードコメントも日本語、識別子は英語。
