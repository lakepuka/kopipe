# リリース手順

## 1. バージョンを上げる

次の 4 ファイルを**同じ値**に揃える（過去の bump コミットもこの 4 点だけ）:

- [package.json](../../package.json) の `version`
- [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json) の `version`
- [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml) の `version`
- [src-tauri/Cargo.lock](../../src-tauri/Cargo.lock) の `kopipe` エントリ
  （`cargo check` を一度走らせれば追従する）

コミットは `chore: bump version to X.Y.Z`（承認を得てから）。

## 2. ビルドと検証

```bash
pnpm verify
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build     # → src-tauri/target/release/bundle/nsis/kopipe_X.Y.Z_x64-setup.exe
```

インストーラの SHA-256 を取る:

```powershell
(Get-FileHash .\kopipe_X.Y.Z_x64-setup.exe -Algorithm SHA256).Hash
```

生成物を**その場で起動して確認**する場合は
[testing.md](testing.md) の「ローカル起動の隔離」に従う（常用アプリを壊さないため）。

## 3. タグと GitHub リリース

タグは `vX.Y.Z`。リリースノートは `## タイトル` ／ `### Changes` ／ `### Install` ／
`### Verify with SHA-256` の構成。

**Install セクションは英日併記が必須**（Changes / Verify は英語のみ）。日本語側は実際の
Windows の UI ラベルを使い、警告が出る理由が**未署名だから**であることを両言語で明記する:

```markdown
### Install
Download `kopipe_X.Y.Z_x64-setup.exe` below and run it. Requires Windows 10/11.

> The installer isn't code-signed yet, so Windows SmartScreen may warn "unknown publisher". Choose **More info → Run anyway**.

**インストール手順**
1. 下の `kopipe_X.Y.Z_x64-setup.exe` をダウンロードして実行します（Windows 10/11 対応）。
2. コード署名を取得していないため、「WindowsによってPCが保護されました」という SmartScreen の警告が出ます。
3. 「詳細情報」をクリックし、「実行」を押すとインストールが始まります。
```

理由: 利用者は日本語話者で、SmartScreen のブロックが最大のつまずき所。押すボタン名と、
マルウェアではなく未署名が原因である説明が要る。

## 4. 公開後

- README の Privacy / Install の記述と実挙動がずれていないか確認する。
- リリースは最新版のみサポート（[SECURITY.md](../../SECURITY.md)）。
