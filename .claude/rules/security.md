# セキュリティ要件

kopipe が扱うのは**ユーザーのクリップボード全履歴**——パスワード、社内文書、個人情報が
そのまま入りうる。「ローカルから出さない」はこのアプリの製品価値そのものなので、
以下は仕様ではなく**不変条件**として扱う。

## 1. ネットワーク（最重要）

- **クリップボードの内容・ファイルパス・統計を外部へ送る実装は書かない。** 例外なし。
- 許可されている外向き通信は 1 つだけ:
  GitHub の最新リリース取得（[api.ts](../../src/services/api.ts) の `fetchLatestRelease`）。
  起動時に 1 回、バージョン番号の比較のみ。設定 `update_check` でオプトアウトできる。
- 新しい fetch / HTTP クライアント / テレメトリ / クラッシュレポータ / 解析 SDK を足さない。
  必要だと感じたら実装前にユーザーへ相談する。
- 変更が README「Privacy」と [SECURITY.md](../../SECURITY.md) の記述と矛盾していないか確認する。
  挙動を変えるなら、その 2 つも同じ PR で更新する。

## 2. WebView の権限

- CSP は `script-src 'self'`（[tauri.conf.json](../../src-tauri/tauri.conf.json)）。緩めない。
  外部 CDN のスクリプト・フォント・`eval` に依存する実装を持ち込まない。
- Tauri の permission は
  [capabilities/default.json](../../src-tauri/capabilities/default.json) に列挙した最小集合。
  `*:default` の追加や広い allow-all で済ませず、**必要な 1 つ**（例:
  `clipboard-manager:allow-write-text`）だけを足す。
- `dangerouslySetInnerHTML` で外部由来の HTML をそのまま描画しない。
  クリップボードの HTML 断片は「HTML ソース表示」のように**テキストとして**見せる。

## 3. データの取り扱い

- SQL は必ずプレースホルダ（`?1` / `rusqlite::params![]`）で組む。
  ユーザー入力（検索語・パス・設定値）を `format!` で SQL に埋め込まない。
  列名など識別子を組み立てる場合は、コード内の固定値のみに限る。
- ユーザーの検索語を正規表現として使う経路がある。パニックさせない（`Regex::new` の失敗は
  エラーとして返す）／不正なパターンでアプリを落とさない。
- **クリップボード本文をログ（`println!` / `console.log`）に出さない。** デバッグ時も。
  出してよいのはパス・件数・種別など内容を含まない情報まで。
- 画像は `%APPDATA%` 配下のアプリ専用ディレクトリに保存する。保存先を設定で変えられるが、
  パスを受け取る処理（`reveal_path` など）は想定外の場所を開かないよう検証する。
- 「履歴を消す」系の操作は本当に消す（DB 行＋孤児になった PNG）。消したつもりで残さない。

## 4. Win32 / OS 連携

- グローバルなキーボードフックと Ctrl+V 送出という、悪用すれば**キーロガーと同型**の
  機能を持っている。フックの中で押されたキーを記録・保存・送信しない。
  用途はトリガー検出（修飾キーの 2 回押し）に限る。
- 自動起動はレジストリ Run キーの自分のエントリのみを触る。他アプリの設定に触れない。

## 5. 依存とリリース

- 依存追加は目的と代替検討を説明してから。`pnpm-lock.yaml` / `Cargo.lock` は必ずコミットする
  （再現可能ビルドとリリース検証の前提）。
- リリース物は CI を通ったコードから作り、SHA-256 をリリースノートに載せる
  （[release.md](release.md)）。インストーラは未署名なので、この checksum が唯一の検証手段。
- 経緯と現状は [docs/code-signing-policy.md](../../docs/code-signing-policy.md) にまとめてある。
- 脆弱性は公開 issue にしない。GitHub Security Advisories 経由（SECURITY.md）。
