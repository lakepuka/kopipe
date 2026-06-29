// clips.rs — クリップ履歴の保存・検索・更新と、React から呼ぶコマンド一式。
//
// Rust メモ:
// - `pub` を付けた物だけが他モジュール（lib.rs など）から見える。
// - #[tauri::command] を付けた pub fn を lib.rs の generate_handler! に登録すると invoke できる。

use rusqlite::{Connection, OptionalExtension};
use tauri::{AppHandle, Emitter, State};

use crate::clip_model::{row_to_clip, Clip, CLIP_COLUMNS};
use crate::db::AppState;

/// プレーンテキストの自動保存（HTML なし）。テスト用の薄いラッパ。
#[cfg(test)]
pub fn record_clip(state: &AppState, content: &str) -> Result<bool, String> {
    record_text(state, content, None)
}

/// テキストの自動保存ヘルパー。同じ内容が既にあれば新規追加せず最新へ繰り上げる
/// （重複排除・ブックマークは維持）。`html` があればリッチ形式として併せて保存する。
/// 保存または繰り上げをしたら true。
pub fn record_text(state: &AppState, content: &str, html: Option<&str>) -> Result<bool, String> {
    if content.trim().is_empty() {
        return Ok(false);
    }
    let conn = state.lock()?;

    // 既存の同一内容を消してから入れ直す（新しい id で確実に最上段へ／重複排除）。
    // 元行のブックマークは引き継ぐ。
    let bookmark = take_existing_bookmark(&conn, "content = ?1 AND kind = 'text'", content)?;
    let content_lower = content.to_lowercase();
    conn.execute(
        "INSERT INTO clips (content, content_lower, created_at, bookmark, kind, html)
         VALUES (?1, ?2, strftime('%s','now'), ?3, 'text', ?4)",
        rusqlite::params![content, content_lower, bookmark, html],
    )
    .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 重複排除の共通処理。`where_clause`（`?1` に `key` を束縛）に一致する既存行が
/// あれば削除し、その行の bookmark を返す。無ければ 0。
/// 「削除 → 入れ直し」で最新行に繰り上げるため、ブックマークだけ引き継ぐ。
fn take_existing_bookmark(conn: &Connection, where_clause: &str, key: &str) -> Result<i64, String> {
    let found: Option<(i64, i64)> = conn
        .query_row(
            &format!(
                "SELECT id, bookmark FROM clips WHERE {where_clause} ORDER BY id DESC LIMIT 1"
            ),
            rusqlite::params![key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some((id, bookmark)) = found {
        conn.execute("DELETE FROM clips WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(bookmark)
    } else {
        Ok(0)
    }
}

/// ファイル/フォルダのコピー（パス一覧）を保存する。同じ内容があれば最上段へ繰り上げ。
/// content は改行区切りのパス。kind = "files"。
pub fn record_files(state: &AppState, content: &str) -> Result<bool, String> {
    if content.trim().is_empty() {
        return Ok(false);
    }
    let conn = state.lock()?;
    let bookmark = take_existing_bookmark(&conn, "content = ?1 AND kind = 'files'", content)?;
    let content_lower = content.to_lowercase();
    conn.execute(
        "INSERT INTO clips (content, content_lower, created_at, bookmark, kind)
         VALUES (?1, ?2, strftime('%s','now'), ?3, 'files')",
        rusqlite::params![content, content_lower, bookmark],
    )
    .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 画像の自動保存ヘルパー。PNG は既に保存済みの前提で、行だけ追加する。
/// 同じ画像（hash 一致）が既にあれば新規追加せず最上段へ繰り上げる。
pub fn record_image(state: &AppState, image_path: &str, hash: &str) -> Result<bool, String> {
    let conn = state.lock()?;
    // 同じ画像（hash 一致）の既存行は消して入れ直す（最上段へ／重複排除）。同 hash は
    // 同じ PNG を指すので image_path は同じ。ブックマークは引き継ぐ。
    let bookmark = take_existing_bookmark(&conn, "hash = ?1 AND kind = 'image'", hash)?;
    conn.execute(
        "INSERT INTO clips (content, content_lower, created_at, bookmark, kind, image_path, hash)
         VALUES ('', '', strftime('%s','now'), ?3, 'image', ?1, ?2)",
        rusqlite::params![image_path, hash, bookmark],
    )
    .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 新しい順に最大 `limit` 件返す。
#[tauri::command]
pub fn list_clips(limit: i64, state: State<AppState>) -> Result<Vec<Clip>, String> {
    let conn = state.lock()?;
    let sql =
        format!("SELECT {CLIP_COLUMNS} FROM clips ORDER BY created_at DESC, id DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let clips = stmt
        .query_map([limit], row_to_clip)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<Clip>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(clips)
}

/// 検索。通常はスペース区切り token の AND 部分一致。use_regex なら正規表現。
/// 画像行は content が空なのでテキスト検索には基本ヒットしない（検索が空なら全件に出る）。
#[tauri::command]
pub fn search_clips(
    query: String,
    limit: i64,
    offset: i64,
    bookmarks_only: bool,
    use_regex: bool,
    state: State<AppState>,
) -> Result<Vec<Clip>, String> {
    // コマンドは「ロックを取って純関数を呼ぶ」だけに薄く保つ（テストしやすさのため）。
    let conn = state.lock()?;
    crate::clip_search::query_clips(&conn, &query, limit, offset, bookmarks_only, use_regex)
}

/// 指定 id の履歴を削除する。画像行なら PNG ファイルも消す。
#[tauri::command]
pub fn delete_clip(id: i64, state: State<AppState>) -> Result<(), String> {
    let conn = state.lock()?;
    delete_clip_inner(&conn, id)
}

/// 削除のコア。行を消し、画像行なら PNG ファイルも消す。テスト可能なよう Connection を受け取る。
fn delete_clip_inner(conn: &Connection, id: i64) -> Result<(), String> {
    // 画像ファイルがあれば消すためにパスを先に取得。
    let path: Option<String> = conn
        .query_row("SELECT image_path FROM clips WHERE id = ?1", [id], |row| {
            row.get::<_, Option<String>>(0)
        })
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();

    conn.execute("DELETE FROM clips WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;

    if let Some(p) = path {
        let _ = std::fs::remove_file(p); // 消せなくても致命的ではない
    }
    Ok(())
}

/// 指定種別の履歴を削除する。kind = "text" | "image"。
/// image のときは DB が参照している画像ファイルだけを消す（保存先の無関係ファイルは触らない）。
#[tauri::command]
pub fn clear_clips(kind: String, app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let conn = state.lock()?;
    if kind == "image" {
        let mut stmt = conn
            .prepare("SELECT image_path FROM clips WHERE kind = 'image' AND image_path IS NOT NULL")
            .map_err(|e| e.to_string())?;
        let paths: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);
        for p in paths {
            let _ = std::fs::remove_file(p);
        }
    }
    conn.execute("DELETE FROM clips WHERE kind = ?1", [&kind])
        .map_err(|e| e.to_string())?;
    drop(conn);
    let _ = app.emit("clips-changed", ());
    Ok(())
}

/// 指定 id のブックマークフラグを反転し、反転後の状態を返す。
#[tauri::command]
pub fn toggle_bookmark(id: i64, state: State<AppState>) -> Result<bool, String> {
    let conn = state.lock()?;

    let changed = conn
        .execute(
            "UPDATE clips SET bookmark = 1 - bookmark WHERE id = ?1",
            [id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("clip {id} not found"));
    }

    let bookmarked: i64 = conn
        .query_row("SELECT bookmark FROM clips WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    Ok(bookmarked != 0)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::clip_search::query_clips;
    use crate::db::{dedupe_clips, open_state};
    use crate::image_storage::enforce_image_limit;

    /// テスト用のインメモリ DB を開く（":memory:" は SQLite のメモリ DB）。
    fn mem() -> AppState {
        open_state(Path::new(":memory:")).unwrap()
    }

    #[test]
    fn record_clip_skips_empty_and_dedupes_to_top() {
        let s = mem();
        assert!(!record_clip(&s, "   ").unwrap()); // 空白のみ → 保存しない
        assert!(record_clip(&s, "hello").unwrap()); // 新規
        record_clip(&s, "world").unwrap();
        // 既存と同じ内容を再コピー → 重複追加せず最上段へ繰り上げ。
        assert!(record_clip(&s, "hello").unwrap());

        let conn = s.lock().unwrap();
        let all = query_clips(&conn, "", 100, 0, false, false).unwrap();
        assert_eq!(all.len(), 2); // hello は 1 件だけ（重複しない）
        assert_eq!(all[0].content, "hello"); // 繰り上がって先頭
    }

    #[test]
    fn record_text_stores_and_dedupes_html() {
        let s = mem();
        record_text(&s, "Tokyo", Some("<b>Tokyo</b>")).unwrap();
        {
            let conn = s.lock().unwrap();
            let all = query_clips(&conn, "", 100, 0, false, false).unwrap();
            assert_eq!(all[0].html.as_deref(), Some("<b>Tokyo</b>"));
        }
        // 同じプレーン内容を別の HTML で再コピー → 1 件のまま、HTML は最新で上書き。
        record_text(&s, "Tokyo", Some("<i>Tokyo</i>")).unwrap();
        let conn = s.lock().unwrap();
        let all = query_clips(&conn, "", 100, 0, false, false).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].html.as_deref(), Some("<i>Tokyo</i>"));
    }

    #[test]
    fn record_clip_dedupe_keeps_bookmark() {
        let s = mem();
        record_clip(&s, "keep me").unwrap();
        {
            let conn = s.lock().unwrap();
            conn.execute(
                "UPDATE clips SET bookmark = 1 WHERE content = 'keep me'",
                [],
            )
            .unwrap();
        }
        record_clip(&s, "other").unwrap();
        record_clip(&s, "keep me").unwrap(); // 再コピー → 繰り上げ（ブックマーク維持）

        let conn = s.lock().unwrap();
        let all = query_clips(&conn, "", 100, 0, false, false).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].content, "keep me");
        assert!(all[0].bookmark); // ブックマークは保たれる
    }

    #[test]
    fn dedupe_clips_collapses_existing_duplicates() {
        let s = mem();
        // 旧ロジック相当の重複を直接作る（content が重複した行）。
        {
            let conn = s.lock().unwrap();
            for (i, content) in ["a", "b", "a", "a"].iter().enumerate() {
                conn.execute(
                    "INSERT INTO clips (content, content_lower, created_at, bookmark, kind)
                     VALUES (?1, ?1, ?2, 0, 'text')",
                    rusqlite::params![content, i as i64],
                )
                .unwrap();
            }
            // 途中の重複 "a" にブックマークを付ける（残す行へ引き継がれるべき）。
            conn.execute("UPDATE clips SET bookmark = 1 WHERE id = 3", [])
                .unwrap();
            dedupe_clips(&conn).unwrap();
        }
        let conn = s.lock().unwrap();
        let all = query_clips(&conn, "", 100, 0, false, false).unwrap();
        assert_eq!(all.len(), 2); // a, b 各 1 件
        let a = all.iter().find(|c| c.content == "a").unwrap();
        assert!(a.bookmark); // ブックマークが残す行へ引き継がれる
    }

    #[test]
    fn search_token_and_is_case_insensitive() {
        let s = mem();
        record_clip(&s, "Hello World").unwrap();
        record_clip(&s, "hello there").unwrap();
        record_clip(&s, "goodbye").unwrap();
        let conn = s.lock().unwrap();

        // "hello" は大文字小文字無視で 2 件ヒット。
        assert_eq!(
            query_clips(&conn, "hello", 100, 0, false, false)
                .unwrap()
                .len(),
            2
        );
        // "hello world" は AND で 1 件のみ。
        let r = query_clips(&conn, "hello world", 100, 0, false, false).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content, "Hello World");
    }

    #[test]
    fn search_empty_returns_all_newest_first() {
        let s = mem();
        record_clip(&s, "a").unwrap();
        record_clip(&s, "b").unwrap();
        let conn = s.lock().unwrap();
        let r = query_clips(&conn, "  ", 100, 0, false, false).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].content, "b"); // 新しい順
    }

    #[test]
    fn search_limit_caps_results() {
        let s = mem();
        for i in 0..5 {
            record_clip(&s, &format!("item{i}")).unwrap();
        }
        let conn = s.lock().unwrap();
        assert_eq!(
            query_clips(&conn, "item", 3, 0, false, false).unwrap().len(),
            3
        );
    }

    #[test]
    fn search_offset_paginates() {
        let s = mem();
        for i in 0..5 {
            record_clip(&s, &format!("item{i}")).unwrap();
        }
        let conn = s.lock().unwrap();
        // 新しい順は item4,3,2,1,0。2 件ずつページング。
        let p0 = query_clips(&conn, "item", 2, 0, false, false).unwrap();
        let p1 = query_clips(&conn, "item", 2, 2, false, false).unwrap();
        let p2 = query_clips(&conn, "item", 2, 4, false, false).unwrap();
        assert_eq!(p0.iter().map(|c| c.content.clone()).collect::<Vec<_>>(), ["item4", "item3"]);
        assert_eq!(p1.iter().map(|c| c.content.clone()).collect::<Vec<_>>(), ["item2", "item1"]);
        assert_eq!(p2.iter().map(|c| c.content.clone()).collect::<Vec<_>>(), ["item0"]);
    }

    #[test]
    fn regex_matches_and_rejects_invalid() {
        let s = mem();
        record_clip(&s, "order-123").unwrap();
        record_clip(&s, "order-abc").unwrap();
        let conn = s.lock().unwrap();

        let r = query_clips(&conn, r"\d{3}", 100, 0, false, true).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content, "order-123");

        // 不正な正規表現は Err。
        assert!(query_clips(&conn, "(", 100, 0, false, true).is_err());
    }

    #[test]
    fn bookmarks_filter_only_returns_bookmarks() {
        let s = mem();
        record_clip(&s, "mark me").unwrap();
        record_clip(&s, "not marked").unwrap();
        {
            let conn = s.lock().unwrap();
            conn.execute(
                "UPDATE clips SET bookmark = 1 WHERE content = 'mark me'",
                [],
            )
            .unwrap();
        } // ロックを解放してから次の lock を取る

        let conn = s.lock().unwrap();
        assert_eq!(query_clips(&conn, "", 100, 0, false, false).unwrap().len(), 2);
        let marked = query_clips(&conn, "", 100, 0, true, false).unwrap();
        assert_eq!(marked.len(), 1);
        assert_eq!(marked[0].content, "mark me");
    }

    #[test]
    fn image_rows_appear_in_list() {
        let s = mem();
        record_clip(&s, "hello").unwrap();
        record_image(&s, "C:/x/abc.png", "abc").unwrap();
        let conn = s.lock().unwrap();
        let all = query_clips(&conn, "", 100, 0, false, false).unwrap();
        assert_eq!(all.len(), 2);
        // 新しい順なので先頭が画像。
        assert_eq!(all[0].kind, "image");
        assert_eq!(all[0].image_path.as_deref(), Some("C:/x/abc.png"));
    }

    // テスト用に指定サイズのダミー画像ファイルを作り、画像行として登録してパスを返す。
    fn make_image(s: &AppState, dir: &std::path::Path, name: &str, bytes: usize) -> String {
        let p = dir.join(name);
        std::fs::write(&p, vec![0u8; bytes]).unwrap();
        let path = p.to_string_lossy().to_string();
        record_image(s, &path, name).unwrap();
        path
    }

    #[test]
    fn record_files_dedups_and_marks_kind() {
        let s = mem();
        assert!(record_files(&s, "C:/a\nC:/b").unwrap());
        assert!(record_files(&s, "C:/c").unwrap());
        // 同じパス一覧を再度 → 重複追加せず最上段へ繰り上げ。
        assert!(record_files(&s, "C:/a\nC:/b").unwrap());

        let conn = s.lock().unwrap();
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM clips WHERE kind = 'files'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(total, 2); // a\nb と c の 2 件（重複しない）
        let top: String = conn
            .query_row(
                "SELECT content FROM clips ORDER BY created_at DESC, id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(top, "C:/a\nC:/b"); // 繰り上がって先頭
    }

    #[test]
    fn enforce_image_limit_removes_oldest_non_bookmarks() {
        let s = mem();
        let dir = std::env::temp_dir().join("kopipe_test_limit_a");
        std::fs::create_dir_all(&dir).unwrap();
        let p1 = make_image(&s, &dir, "a.png", 100);
        let p2 = make_image(&s, &dir, "b.png", 100);
        let p3 = make_image(&s, &dir, "c.png", 100);

        // 合計 300、上限 150 → 古い順に p1,p2 を削除して 100 に。
        let removed = enforce_image_limit(&s, 150).unwrap();
        assert_eq!(removed, 200);
        assert!(!std::path::Path::new(&p1).exists());
        assert!(!std::path::Path::new(&p2).exists());
        assert!(std::path::Path::new(&p3).exists());

        let conn = s.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clips WHERE kind='image'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn enforce_image_limit_keeps_bookmarks() {
        let s = mem();
        let dir = std::env::temp_dir().join("kopipe_test_limit_b");
        std::fs::create_dir_all(&dir).unwrap();
        let p1 = make_image(&s, &dir, "bookmarked.png", 100); // 最古
        let p2 = make_image(&s, &dir, "x.png", 100);
        // 最古をブックマークにする。
        s.lock()
            .unwrap()
            .execute("UPDATE clips SET bookmark = 1 WHERE image_path = ?1", [&p1])
            .unwrap();

        // 上限 50（合計 200）。ブックマークは消さないので、非ブックマークの p2 だけ削除。
        enforce_image_limit(&s, 50).unwrap();
        assert!(std::path::Path::new(&p1).exists()); // ブックマークは残る
        assert!(!std::path::Path::new(&p2).exists());
    }

    #[test]
    fn delete_removes_row_and_image_file() {
        let s = mem();
        // 一時ファイルを作って画像行として登録。
        let path = std::env::temp_dir().join("kopipe_test_delete.png");
        std::fs::write(&path, b"dummy").unwrap();
        let path_str = path.to_string_lossy().to_string();
        record_image(&s, &path_str, "deadbeef").unwrap();

        let id: i64 = {
            let conn = s.lock().unwrap();
            conn.query_row("SELECT id FROM clips WHERE hash = 'deadbeef'", [], |r| {
                r.get(0)
            })
            .unwrap()
        };

        {
            let conn = s.lock().unwrap();
            delete_clip_inner(&conn, id).unwrap();
            // 行が消えている。
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM clips WHERE id = ?1", [id], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(count, 0);
        }
        // PNG ファイルも消えている。
        assert!(!path.exists());
    }
}
