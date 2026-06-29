// settings.rs — アプリ設定の永続化（SQLite の key/value テーブル）。
//
// ロジック（get_all / set）は &Connection を受け取る純関数にして単体テスト可能にし、
// #[tauri::command] はそれを呼ぶだけの薄いラッパにする。

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};
use tauri::{AppHandle, Emitter, State};

use crate::db::AppState;

/// settings テーブルを用意する（open_state から呼ばれる）。
pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
}

/// 全設定を key→value のマップで返す。
fn get_all(conn: &Connection) -> Result<HashMap<String, String>, String> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (k, v) = r.map_err(|e| e.to_string())?;
        map.insert(k, v);
    }
    Ok(map)
}

/// 1 件の設定を upsert する（あれば更新、無ければ挿入）。
fn set(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 1 件だけ取得する（無ければ None）。他モジュール（system 等）から使う。
pub fn get_one(state: &AppState, key: &str) -> Option<String> {
    let conn = state.lock().ok()?;
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
        r.get::<_, String>(0)
    })
    .optional()
    .ok()
    .flatten()
}

/// 1 件 upsert する（他モジュールから使う）。
pub fn set_value(state: &AppState, key: &str, value: &str) -> Result<(), String> {
    let conn = state.lock()?;
    set(&conn, key, value)
}

/// 全設定を消す（初期設定に戻す）。
pub fn clear(state: &AppState) -> Result<(), String> {
    let conn = state.lock()?;
    conn.execute("DELETE FROM settings", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<HashMap<String, String>, String> {
    let conn = state.lock()?;
    get_all(&conn)
}

#[tauri::command]
pub fn set_setting(
    key: String,
    value: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    {
        let conn = state.lock()?;
        set(&conn, &key, &value)?;
    }
    // 言語が変わったらトレイメニューの文言も追従させる（Rust 側で組むため）。
    #[cfg(desktop)]
    if key == "lang" {
        crate::system::window::apply_tray_lang(&app, &value);
    }
    // 全ウィンドウに通知して、開いている画面が即反映できるようにする。
    let _ = app.emit("settings-changed", ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::db::open_state;

    #[test]
    fn set_then_get_roundtrips_and_upserts() {
        let s = open_state(Path::new(":memory:")).unwrap();
        let conn = s.lock().unwrap();

        assert!(get_all(&conn).unwrap().is_empty());

        set(&conn, "theme", "dark").unwrap();
        set(&conn, "theme", "light").unwrap(); // 同じ key は上書き
        set(&conn, "width", "400").unwrap();

        let all = get_all(&conn).unwrap();
        assert_eq!(all.get("theme").map(String::as_str), Some("light"));
        assert_eq!(all.get("width").map(String::as_str), Some("400"));
        assert_eq!(all.len(), 2);
    }
}
