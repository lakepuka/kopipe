// db.rs — SQLite 接続状態と起動時マイグレーション。

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

/// アプリ全体で共有する状態。
/// SQLite の `Connection` はスレッド間で同時に使えない（Sync でない）ため、
/// `Mutex` で包んで「同時に触るのは一人だけ」を保証する。
pub struct AppState {
    db: Mutex<Connection>,
}

impl AppState {
    /// 他モジュールから DB 接続を借りるためのアクセサ。
    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.db.lock().map_err(|e| e.to_string())
    }
}

/// clips テーブルの列名一覧（PRAGMA table_info の 2 列目=index 1 が列名）。
fn clip_columns(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("PRAGMA table_info(clips)")?;
    let cols = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(cols)
}

/// 列が無ければ ALTER TABLE で追加する（既存 DB のマイグレーション用）。
fn ensure_column(conn: &Connection, col: &str, decl: &str) -> rusqlite::Result<()> {
    if !clip_columns(conn)?.iter().any(|name| name == col) {
        conn.execute(&format!("ALTER TABLE clips ADD COLUMN {col} {decl}"), [])?;
    }
    Ok(())
}

/// 旧列名が残っていて新列名が無ければ ALTER TABLE で改名する（マイグレーション用）。
fn rename_column_if_present(conn: &Connection, old: &str, new: &str) -> rusqlite::Result<()> {
    let cols = clip_columns(conn)?;
    if cols.iter().any(|c| c == old) && !cols.iter().any(|c| c == new) {
        conn.execute(
            &format!("ALTER TABLE clips RENAME COLUMN {old} TO {new}"),
            [],
        )?;
    }
    Ok(())
}

/// DB を開き、テーブルを用意して AppState を作る。
pub fn open_state(db_path: &Path) -> rusqlite::Result<AppState> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clips (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            content      TEXT    NOT NULL,
            content_lower TEXT   NOT NULL,
            created_at   INTEGER NOT NULL,
            bookmark     INTEGER NOT NULL DEFAULT 0
        );",
    )?;
    // 旧列 favorite を bookmark へ改名（既存 DB のマイグレーション）。
    rename_column_if_present(&conn, "favorite", "bookmark")?;
    // 画像対応の列を（無ければ）追加する。
    ensure_column(&conn, "kind", "TEXT NOT NULL DEFAULT 'text'")?;
    ensure_column(&conn, "image_path", "TEXT")?;
    ensure_column(&conn, "hash", "TEXT")?;
    ensure_column(&conn, "html", "TEXT")?;
    // 既存データの重複を一度だけ整理する。
    dedupe_clips(&conn)?;
    // 設定テーブルを用意する。
    crate::settings::init(&conn)?;
    Ok(AppState {
        db: Mutex::new(conn),
    })
}

/// 既存データの重複を整理する（起動時に一度）。同一内容（テキスト/ファイルは content、
/// 画像は hash）の行は最新（id 最大）の 1 件だけ残す。
pub(crate) fn dedupe_clips(conn: &Connection) -> rusqlite::Result<()> {
    // --- テキスト/ファイル: (kind, content) でグループ化 ---
    // 残す行にブックマークを引き継ぐ。
    conn.execute(
        "UPDATE clips SET bookmark = 1
         WHERE kind IN ('text','files')
           AND id IN (SELECT MAX(id) FROM clips WHERE kind IN ('text','files') GROUP BY kind, content)
           AND EXISTS (SELECT 1 FROM clips d
                       WHERE d.kind = clips.kind AND d.content = clips.content AND d.bookmark = 1)",
        [],
    )?;
    conn.execute(
        "DELETE FROM clips
         WHERE kind IN ('text','files')
           AND id NOT IN (SELECT MAX(id) FROM clips WHERE kind IN ('text','files') GROUP BY kind, content)",
        [],
    )?;

    // --- 画像: hash でグループ化（同 hash は同じ PNG を指すのでファイルは消さない） ---
    conn.execute(
        "UPDATE clips SET bookmark = 1
         WHERE kind = 'image' AND hash IS NOT NULL
           AND id IN (SELECT MAX(id) FROM clips WHERE kind = 'image' AND hash IS NOT NULL GROUP BY hash)
           AND EXISTS (SELECT 1 FROM clips d
                       WHERE d.kind = 'image' AND d.hash = clips.hash AND d.bookmark = 1)",
        [],
    )?;
    conn.execute(
        "DELETE FROM clips
         WHERE kind = 'image' AND hash IS NOT NULL
           AND id NOT IN (SELECT MAX(id) FROM clips WHERE kind = 'image' AND hash IS NOT NULL GROUP BY hash)",
        [],
    )?;
    Ok(())
}
