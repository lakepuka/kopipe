// clip_search.rs — 履歴検索の SQL 組み立てと正規表現検索。

use rusqlite::{params_from_iter, Connection};

use crate::clip_model::{attach_tags, row_to_clip, Clip, CLIP_COLUMNS};

/// 検索文字列の `tag:` 接頭辞。`tag:仕事,SQL` のようにカンマ区切りで複数指定できる。
const TAG_PREFIX: &str = "tag:";

/// LIKE 用パターンを作る。`%` `_` `\` は LIKE のメタ文字なのでエスケープする。
fn like_pattern(token: &str) -> String {
    let escaped = token
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{}%", escaped)
}

/// 検索文字列を「本文検索の部分」と「タグ絞り込み」に分けた結果。
pub(crate) struct ParsedQuery {
    /// `tag:` トークンを取り除いた残り。正規表現モードではこれをそのまま使う。
    pub(crate) text: String,
    /// 絞り込むタグ名（AND）。
    pub(crate) tags: Vec<String>,
}

/// 検索文字列から `tag:` トークンを切り出す。
/// `tag:` が 1 つも無ければ text は原文のまま（正規表現をトークン分割で壊さないため）。
pub(crate) fn parse_query(query: &str) -> ParsedQuery {
    let has_tag = query
        .split_whitespace()
        .any(|t| t.to_lowercase().starts_with(TAG_PREFIX));
    if !has_tag {
        return ParsedQuery {
            text: query.to_string(),
            tags: Vec::new(),
        };
    }

    let mut rest: Vec<&str> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    for token in query.split_whitespace() {
        if token.to_lowercase().starts_with(TAG_PREFIX) {
            for name in token[TAG_PREFIX.len()..].split(',') {
                let name = name.trim();
                // 大小文字を無視した重複は 1 つにまとめる（tag:a,A で 2 条件にしない）。
                if !name.is_empty() && !tags.iter().any(|t| t.to_lowercase() == name.to_lowercase())
                {
                    tags.push(name.to_string());
                }
            }
        } else {
            rest.push(token);
        }
    }
    ParsedQuery {
        text: rest.join(" "),
        tags,
    }
}

/// 指定タグが付いているか。`tags.name` は NOCASE なので大小文字は無視される。
const TAG_MATCH: &str = "EXISTS (SELECT 1 FROM clip_tags ct JOIN tags t ON t.id = ct.tag_id
                                 WHERE ct.clip_id = clips.id AND t.name = ?)";

/// 語が本文・説明・タグ名のどれかに部分一致するか。? を 3 つ使う（同じパターンを 3 回束縛）。
const TOKEN_MATCH: &str = "(content_lower LIKE ? ESCAPE '\\'
                           OR lower(IFNULL(note, '')) LIKE ? ESCAPE '\\'
                           OR EXISTS (SELECT 1 FROM clip_tags ct JOIN tags t ON t.id = ct.tag_id
                                      WHERE ct.clip_id = clips.id
                                        AND lower(t.name) LIKE ? ESCAPE '\\'))";

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
    let parsed = parse_query(query);

    if use_regex {
        return search_by_regex(conn, &parsed, limit, offset, bookmarks_only);
    }

    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    for token in parsed.text.split_whitespace() {
        let pattern = like_pattern(&token.to_lowercase());
        conditions.push(TOKEN_MATCH.to_string());
        // TOKEN_MATCH の ? は 3 つ。順番どおりに同じパターンを積む。
        params.push(pattern.clone());
        params.push(pattern.clone());
        params.push(pattern);
    }
    for tag in &parsed.tags {
        conditions.push(TAG_MATCH.to_string());
        params.push(tag.clone());
    }
    if bookmarks_only {
        conditions.push("bookmark = 1".to_string());
    }
    let where_clause = if conditions.is_empty() {
        "1=1".to_string()
    } else {
        conditions.join(" AND ")
    };

    // limit/offset は自分で持つ i64 なので埋め込み可。ユーザー入力は必ず ? で束縛。
    let sql = format!(
        "SELECT {CLIP_COLUMNS} FROM clips
         WHERE {where_clause}
         ORDER BY created_at DESC, id DESC
         LIMIT {limit} OFFSET {offset}"
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut clips = stmt
        .query_map(params_from_iter(params.iter()), row_to_clip)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<Clip>, _>>()
        .map_err(|e| e.to_string())?;
    attach_tags(conn, &mut clips)?;
    Ok(clips)
}

/// 正規表現検索。SQLite は正規表現を扱えないので新しい順に走査し Rust 側で判定する。
/// タグ絞り込みだけは SQL 側で効かせ、走査対象を減らす。
fn search_by_regex(
    conn: &Connection,
    parsed: &ParsedQuery,
    limit: i64,
    offset: i64,
    bookmarks_only: bool,
) -> Result<Vec<Clip>, String> {
    use regex::RegexBuilder;

    let re = RegexBuilder::new(&parsed.text)
        .case_insensitive(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();
    for tag in &parsed.tags {
        conditions.push(TAG_MATCH.to_string());
        params.push(tag.clone());
    }
    if bookmarks_only {
        conditions.push("bookmark = 1".to_string());
    }
    let where_clause = if conditions.is_empty() {
        "1=1".to_string()
    } else {
        conditions.join(" AND ")
    };
    let sql = format!(
        "SELECT {CLIP_COLUMNS} FROM clips
         WHERE {where_clause}
         ORDER BY created_at DESC, id DESC"
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params_from_iter(params.iter()), row_to_clip)
        .map_err(|e| e.to_string())?;

    // SQLite では OFFSET できないので、マッチ済み件数を数えて offset 件を読み飛ばす。
    let mut matched = 0i64;
    let mut clips = Vec::new();
    for r in rows {
        let c = r.map_err(|e| e.to_string())?;
        // 説明もマッチ対象にする（行の右側に出ている文字で引ける、を保つ）。
        if re.is_match(&c.content) || c.note.as_deref().is_some_and(|n| re.is_match(n)) {
            if matched >= offset {
                clips.push(c);
                if clips.len() as i64 >= limit {
                    break;
                }
            }
            matched += 1;
        }
    }
    attach_tags(conn, &mut clips)?;
    Ok(clips)
}

#[cfg(test)]
mod tests {
    use super::{like_pattern, parse_query};

    #[test]
    fn like_pattern_escapes_metachars() {
        assert_eq!(like_pattern("a_b"), "%a\\_b%");
        assert_eq!(like_pattern("50%off"), "%50\\%off%");
        assert_eq!(like_pattern("a\\b"), "%a\\\\b%");
    }

    #[test]
    fn parse_query_without_tag_keeps_text_as_is() {
        // 正規表現をトークン分割で壊さないよう、tag: が無ければ原文のまま。
        let p = parse_query(r"a  b\s+c");
        assert_eq!(p.text, r"a  b\s+c");
        assert!(p.tags.is_empty());
    }

    #[test]
    fn parse_query_splits_tags_and_text() {
        let p = parse_query("請求書 tag:仕事,SQL 控え");
        assert_eq!(p.text, "請求書 控え");
        assert_eq!(p.tags, vec!["仕事", "SQL"]);
    }

    #[test]
    fn parse_query_merges_multiple_tag_tokens_and_dedupes() {
        let p = parse_query("TAG:a tag:A,b");
        assert_eq!(p.text, "");
        assert_eq!(p.tags, vec!["a", "b"]); // 大小違いは 1 つ扱い、接頭辞も大小無視
    }

    #[test]
    fn parse_query_ignores_empty_names() {
        let p = parse_query("tag: tag:,x,");
        assert_eq!(p.tags, vec!["x"]);
    }
}
