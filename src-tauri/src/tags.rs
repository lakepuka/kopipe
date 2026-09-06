// tags.rs — 説明（note）とタグの永続化。ロジックは Connection を受け取る純関数側に置く。
//
// タグ名は「空白とカンマを含まない」ことを不変条件にする。検索文字列の `tag:a,b` 構文が
// 空白区切り・カンマ区切りなので、名前にそれらが混ざると絞り込みを表現できなくなるため。
// UI 側でも空白・カンマは区切りとして扱うが、ここでも防御的に分割する。

use std::collections::HashMap;

use rusqlite::{params_from_iter, Connection};
use tauri::{AppHandle, Emitter, State};

use crate::db::AppState;

/// タグ名の最大長。長すぎる名前は行の右側と絞り込みバーの見た目を壊すので切り詰める。
const MAX_TAG_LEN: usize = 32;

/// 入力文字列をタグ名の並びへ正規化する。空白・カンマで分割し、空を捨て、
/// 長すぎるものを切り詰め、大小文字を無視して重複を除く（先に現れた表記を残す）。
pub(crate) fn normalize_tags(input: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in input {
        for part in raw.split([',', ' ', '\t', '\n', '\r']) {
            let name: String = part.trim().chars().take(MAX_TAG_LEN).collect();
            if name.is_empty() {
                continue;
            }
            if !out.iter().any(|e| e.to_lowercase() == name.to_lowercase()) {
                out.push(name);
            }
        }
    }
    out
}

/// 説明を保存する。空白のみ・空文字は「説明なし」として NULL に戻す。
pub(crate) fn set_note(conn: &Connection, id: i64, note: &str) -> Result<(), String> {
    let trimmed = note.trim();
    let value = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    };
    conn.execute(
        "UPDATE clips SET note = ?1 WHERE id = ?2",
        rusqlite::params![value, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// タグ名から tags 行の id を得る（無ければ作る）。NOCASE なので大小違いは同じ行になる。
fn tag_id(conn: &Connection, name: &str) -> Result<i64, String> {
    conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [name])
        .map_err(|e| e.to_string())?;
    conn.query_row("SELECT id FROM tags WHERE name = ?1", [name], |r| r.get(0))
        .map_err(|e| e.to_string())
}

/// クリップのタグを丸ごと差し替える。空配列なら全部外す。
/// 差し替えで参照されなくなったタグは一覧からも消す（孤児タグを残さない）。
pub(crate) fn set_tags(conn: &Connection, id: i64, names: &[String]) -> Result<(), String> {
    let names = normalize_tags(names);
    conn.execute("DELETE FROM clip_tags WHERE clip_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    for name in &names {
        let tid = tag_id(conn, name)?;
        conn.execute(
            "INSERT OR IGNORE INTO clip_tags (clip_id, tag_id) VALUES (?1, ?2)",
            [id, tid],
        )
        .map_err(|e| e.to_string())?;
    }
    prune_orphans(conn).map_err(|e| e.to_string())?;
    Ok(())
}

/// 指定 id 群のタグをそのまま付け直す（同じ内容を再コピーしたときの引き継ぎ用）。
pub(crate) fn attach_tag_ids(
    conn: &Connection,
    clip_id: i64,
    tag_ids: &[i64],
) -> Result<(), String> {
    for tid in tag_ids {
        conn.execute(
            "INSERT OR IGNORE INTO clip_tags (clip_id, tag_id) VALUES (?1, ?2)",
            [clip_id, *tid],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 指定クリップに付いている tags.id 一覧。
pub(crate) fn tag_ids_of(conn: &Connection, clip_id: i64) -> Result<Vec<i64>, String> {
    let mut stmt = conn
        .prepare("SELECT tag_id FROM clip_tags WHERE clip_id = ?1")
        .map_err(|e| e.to_string())?;
    let ids = stmt
        .query_map([clip_id], |r| r.get::<_, i64>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(ids)
}

/// 消えたクリップの紐付けと、どこからも参照されていないタグを掃除する。
pub(crate) fn prune_orphans(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM clip_tags WHERE clip_id NOT IN (SELECT id FROM clips)",
        [],
    )?;
    conn.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM clip_tags)",
        [],
    )?;
    Ok(())
}

/// 使用中のタグ名一覧（名前順）。どこからも参照されていないタグは含まない。
pub(crate) fn all_tags(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT t.name FROM tags t
             WHERE EXISTS (SELECT 1 FROM clip_tags ct WHERE ct.tag_id = t.id)
             ORDER BY t.name COLLATE NOCASE",
        )
        .map_err(|e| e.to_string())?;
    let names = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(names)
}

/// 複数クリップのタグをまとめて引く（1 クエリ）。検索結果に添えて返すために使う。
/// group_concat は並び順が保証されないので、SQL で名前順に並べて Rust 側で束ねる。
pub(crate) fn tags_for_clips(
    conn: &Connection,
    ids: &[i64],
) -> Result<HashMap<i64, Vec<String>>, String> {
    let mut map: HashMap<i64, Vec<String>> = HashMap::new();
    if ids.is_empty() {
        return Ok(map);
    }
    // 埋め込むのは「?」の数だけ。id 自体は必ず束縛する。
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT ct.clip_id, t.name FROM clip_tags ct
         JOIN tags t ON t.id = ct.tag_id
         WHERE ct.clip_id IN ({placeholders})
         ORDER BY t.name COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params_from_iter(ids.iter()), |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (clip_id, name) = row.map_err(|e| e.to_string())?;
        map.entry(clip_id).or_default().push(name);
    }
    Ok(map)
}

// --- コマンド（ロックを取って純関数を呼び、変更を通知するだけの薄いラッパ） ---

/// 説明を保存する。空文字なら説明を消す。
#[tauri::command]
pub fn set_clip_note(
    id: i64,
    note: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    {
        let conn = state.lock()?;
        set_note(&conn, id, &note)?;
    } // emit の前にロックを手放す
    let _ = app.emit("clips-changed", ());
    Ok(())
}

/// タグを差し替える。空配列なら全部外す。
#[tauri::command]
pub fn set_clip_tags(
    id: i64,
    tags: Vec<String>,
    app: AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    {
        let conn = state.lock()?;
        set_tags(&conn, id, &tags)?;
    }
    let _ = app.emit("clips-changed", ());
    Ok(())
}

/// 使用中のタグ一覧（名前順）。
#[tauri::command]
pub fn list_tags(state: State<AppState>) -> Result<Vec<String>, String> {
    let conn = state.lock()?;
    all_tags(&conn)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::clips::record_clip;
    use crate::db::open_state;

    fn mem() -> AppState {
        open_state(Path::new(":memory:")).unwrap()
    }

    /// content から id を引く小道具。
    fn id_of(conn: &Connection, content: &str) -> i64 {
        conn.query_row("SELECT id FROM clips WHERE content = ?1", [content], |r| {
            r.get(0)
        })
        .unwrap()
    }

    #[test]
    fn normalize_splits_and_dedupes() {
        let input = vec![
            "仕事, SQL".to_string(),
            " sql ".to_string(),
            "  ".to_string(),
        ];
        // カンマ・空白で分割し、大小文字を無視して重複を除く（先勝ち）。
        assert_eq!(normalize_tags(&input), vec!["仕事", "SQL"]);
    }

    #[test]
    fn normalize_truncates_long_names() {
        let long = "a".repeat(50);
        assert_eq!(normalize_tags(&[long])[0].chars().count(), MAX_TAG_LEN);
    }

    #[test]
    fn empty_note_clears_it() {
        let s = mem();
        record_clip(&s, "hello").unwrap();
        let conn = s.lock().unwrap();
        let id = id_of(&conn, "hello");

        set_note(&conn, id, "  メモ  ").unwrap();
        let note: Option<String> = conn
            .query_row("SELECT note FROM clips WHERE id = ?1", [id], |r| r.get(0))
            .unwrap();
        assert_eq!(note.as_deref(), Some("メモ")); // 前後の空白は落とす

        set_note(&conn, id, "   ").unwrap();
        let note: Option<String> = conn
            .query_row("SELECT note FROM clips WHERE id = ?1", [id], |r| r.get(0))
            .unwrap();
        assert!(note.is_none()); // 空文字は「説明なし」
    }

    #[test]
    fn set_tags_replaces_and_prunes_orphans() {
        let s = mem();
        record_clip(&s, "hello").unwrap();
        let conn = s.lock().unwrap();
        let id = id_of(&conn, "hello");

        set_tags(&conn, id, &["仕事".into(), "SQL".into()]).unwrap();
        assert_eq!(all_tags(&conn).unwrap(), vec!["SQL", "仕事"]);

        // SQL を外すと、他のどこからも使われていないので一覧からも消える。
        set_tags(&conn, id, &["仕事".into()]).unwrap();
        assert_eq!(all_tags(&conn).unwrap(), vec!["仕事"]);

        // 全部外せば一覧は空。
        set_tags(&conn, id, &[]).unwrap();
        assert!(all_tags(&conn).unwrap().is_empty());
    }

    #[test]
    fn tag_still_listed_while_another_clip_uses_it() {
        let s = mem();
        record_clip(&s, "a").unwrap();
        record_clip(&s, "b").unwrap();
        let conn = s.lock().unwrap();
        let (a, b) = (id_of(&conn, "a"), id_of(&conn, "b"));

        set_tags(&conn, a, &["共有".into()]).unwrap();
        set_tags(&conn, b, &["共有".into()]).unwrap();
        set_tags(&conn, a, &[]).unwrap();
        // b がまだ使っているので残る。
        assert_eq!(all_tags(&conn).unwrap(), vec!["共有"]);
    }

    #[test]
    fn deleting_a_clip_drops_its_tag() {
        let s = mem();
        record_clip(&s, "hello").unwrap();
        let conn = s.lock().unwrap();
        let id = id_of(&conn, "hello");
        set_tags(&conn, id, &["一時".into()]).unwrap();

        conn.execute("DELETE FROM clips WHERE id = ?1", [id])
            .unwrap();
        prune_orphans(&conn).unwrap();
        assert!(all_tags(&conn).unwrap().is_empty());
    }
}
