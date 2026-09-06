// clip_model.rs — クリップ履歴のデータ型と SQLite 行マッピング。

use rusqlite::Connection;
use serde::Serialize;

/// フロント(React)へ返す 1 件分のデータ。
/// kind が "text" なら content、"image" なら image_path を使う。
#[derive(Serialize)]
pub struct Clip {
    pub(crate) id: i64,
    pub(crate) content: String,
    pub(crate) created_at: i64, // Unix 秒
    pub(crate) bookmark: bool,
    pub(crate) kind: String,               // "text" | "image"
    pub(crate) image_path: Option<String>, // 画像のときだけ PNG のパス
    pub(crate) html: Option<String>,       // リッチ形式があれば HTML 断片（なければ NULL）
    pub(crate) note: Option<String>,       // ユーザーが付けた説明（なければ NULL）
    pub(crate) tags: Vec<String>,          // 付いているタグ（名前順）。別テーブルから後付け。
}

// SELECT で取り出す列順。row_to_clip と必ず一致させること。
// tags は clip_tags との JOIN が要るので列には含めず、attach_tags で後から埋める。
pub(crate) const CLIP_COLUMNS: &str =
    "id, content, created_at, bookmark, kind, image_path, html, note";

/// 1 行を Clip に変換する共通マッパー。列を増やすときはここと CLIP_COLUMNS だけ直す。
pub(crate) fn row_to_clip(row: &rusqlite::Row) -> rusqlite::Result<Clip> {
    Ok(Clip {
        id: row.get(0)?,
        content: row.get(1)?,
        created_at: row.get(2)?,
        bookmark: row.get::<_, i64>(3)? != 0, // SQLite に bool はないので 0/1 を変換
        kind: row.get(4)?,
        image_path: row.get(5)?,
        html: row.get(6)?,
        note: row.get(7)?,
        tags: Vec::new(), // attach_tags で埋める
    })
}

/// 取得した行にタグを付ける。件数によらず 1 クエリで引く（行ごとに引くと N+1 になる）。
pub(crate) fn attach_tags(conn: &Connection, clips: &mut [Clip]) -> Result<(), String> {
    let ids: Vec<i64> = clips.iter().map(|c| c.id).collect();
    let mut map = crate::tags::tags_for_clips(conn, &ids)?;
    for c in clips.iter_mut() {
        if let Some(names) = map.remove(&c.id) {
            c.tags = names;
        }
    }
    Ok(())
}
