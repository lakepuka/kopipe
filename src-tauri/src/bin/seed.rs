// seed — 開発用にテストクリップを大量投入するスタンドアロンコマンド。
//
//   pnpm run seed            … 既定 1000 件
//   pnpm run seed 5000       … 件数を指定
//   cargo run --bin seed -- 5000   （src-tauri から直接）
//
// アプリ本体と同じ %APPDATA%\io.github.lakepuka.kopipe\kopipe.db を開いて
// INSERT するだけ。リリースバイナリには含まれない（cargo の bin として別物）。

use std::path::PathBuf;

use kopipe_lib::db::open_state;

// tauri.conf.json の identifier と一致させる（app_data_dir のフォルダ名）。
const IDENTIFIER: &str = "io.github.lakepuka.kopipe";

fn db_path() -> PathBuf {
    // Windows: %APPDATA%\<identifier>\kopipe.db
    let appdata = std::env::var("APPDATA").expect("APPDATA が未設定です（Windows 専用）");
    PathBuf::from(appdata).join(IDENTIFIER).join("kopipe.db")
}

fn main() {
    let count: i64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);

    let path = db_path();
    // open_state がテーブル作成・マイグレーションまで面倒を見る。
    let state = open_state(&path).expect("DB を開けませんでした");
    let mut conn = state.lock().expect("DB ロック失敗");

    let tx = conn.transaction().expect("トランザクション開始失敗");
    for i in 0..count {
        let content = format!("test clip {i} — sample searchable text item{}", i % 100);
        let lower = content.to_lowercase();
        // created_at を 1 秒ずつずらして、新しい順の並びが自然に見えるようにする。
        tx.execute(
            "INSERT INTO clips (content, content_lower, created_at, bookmark, kind)
             VALUES (?1, ?2, strftime('%s','now') - ?3, 0, 'text')",
            rusqlite::params![content, lower, i],
        )
        .expect("INSERT 失敗");
    }
    tx.commit().expect("commit 失敗");

    println!("seeded {count} clips into {}", path.display());
}
