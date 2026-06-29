// clip_search.rs — 履歴検索の SQL 組み立てと正規表現検索。

use rusqlite::{params_from_iter, Connection};

use crate::clip_model::{row_to_clip, Clip, CLIP_COLUMNS};

/// LIKE 用パターンを作る。`%` `_` `\` は LIKE のメタ文字なのでエスケープする。
fn like_pattern(token: &str) -> String {
    let escaped = token
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{}%", escaped)
}

/// 検索のコア。Connection を直接受け取るので `:memory:` DB で単体テストできる。
/// `offset` は無限スクロール用に「先頭から何件読み飛ばすか」。
pub(crate) fn query_clips(
    conn: &Connection,
    query: &str,
    limit: i64,
    offset: i64,
    bookmarks_only: bool,
    use_regex: bool,
) -> Result<Vec<Clip>, String> {
    if use_regex {
        return search_by_regex(conn, query, limit, offset, bookmarks_only);
    }

    let tokens: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();

    let mut conditions: Vec<&str> = tokens
        .iter()
        .map(|_| "content_lower LIKE ? ESCAPE '\\'")
        .collect();
    if bookmarks_only {
        conditions.push("bookmark = 1");
    }
    let where_clause = if conditions.is_empty() {
        "1=1".to_string()
    } else {
        conditions.join(" AND ")
    };

    // limit/offset は自分で持つ i64 なので埋め込み可。token はユーザー入力なので必ず ? で束縛。
    let sql = format!(
        "SELECT {CLIP_COLUMNS} FROM clips
         WHERE {where_clause}
         ORDER BY created_at DESC, id DESC
         LIMIT {limit} OFFSET {offset}"
    );

    let patterns: Vec<String> = tokens.iter().map(|t| like_pattern(t)).collect();
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let clips = stmt
        .query_map(params_from_iter(patterns.iter()), row_to_clip)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<Clip>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(clips)
}

/// 正規表現検索。SQLite は正規表現を扱えないので新しい順に走査し Rust 側で判定する。
fn search_by_regex(
    conn: &Connection,
    pattern: &str,
    limit: i64,
    offset: i64,
    bookmarks_only: bool,
) -> Result<Vec<Clip>, String> {
    use regex::RegexBuilder;

    let re = RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .map_err(|e| e.to_string())?;

    let where_clause = if bookmarks_only {
        "bookmark = 1"
    } else {
        "1=1"
    };
    let sql = format!(
        "SELECT {CLIP_COLUMNS} FROM clips
         WHERE {where_clause}
         ORDER BY created_at DESC, id DESC"
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_clip).map_err(|e| e.to_string())?;

    // SQLite では OFFSET できないので、マッチ済み件数を数えて offset 件を読み飛ばす。
    let mut matched = 0i64;
    let mut clips = Vec::new();
    for r in rows {
        let c = r.map_err(|e| e.to_string())?;
        if re.is_match(&c.content) {
            if matched >= offset {
                clips.push(c);
                if clips.len() as i64 >= limit {
                    break;
                }
            }
            matched += 1;
        }
    }
    Ok(clips)
}

#[cfg(test)]
mod tests {
    use super::like_pattern;

    #[test]
    fn like_pattern_escapes_metachars() {
        assert_eq!(like_pattern("a_b"), "%a\\_b%");
        assert_eq!(like_pattern("50%off"), "%50\\%off%");
        assert_eq!(like_pattern("a\\b"), "%a\\\\b%");
    }
}
