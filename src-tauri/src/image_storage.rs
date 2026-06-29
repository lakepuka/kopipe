// image_storage.rs — 画像履歴の読み出し、保存先解決、容量制限。

use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::AppState;

/// 指定 id の画像ファイルパスを返す（画像行のみ）。
pub fn image_path(state: &AppState, id: i64) -> Result<String, String> {
    let conn = state.lock()?;
    conn.query_row(
        "SELECT image_path FROM clips WHERE id = ?1 AND kind = 'image'",
        [id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

/// 画像をデータURL（data:image/png;base64,...）にして返す。フロントの <img src> 用。
#[tauri::command]
pub fn image_data_url(id: i64, state: State<AppState>) -> Result<String, String> {
    use base64::Engine;

    let path = image_path(&state, id)?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}

/// 画像の保存先フォルダを解決する。設定 "image_dir" があればそれ、無ければ app_data/images。
pub fn image_dir(app: &AppHandle) -> std::path::PathBuf {
    let custom = crate::settings::get_one(&app.state::<AppState>(), "image_dir")
        .filter(|s| !s.trim().is_empty());
    if let Some(p) = custom {
        return std::path::PathBuf::from(p);
    }
    app.path()
        .app_data_dir()
        .map(|d| d.join("images"))
        .unwrap_or_else(|_| std::path::PathBuf::from("images"))
}

/// 解決済みの画像保存先パスを返す（設定画面の表示用）。
#[tauri::command]
pub fn image_dir_path(app: AppHandle) -> Result<String, String> {
    Ok(image_dir(&app).to_string_lossy().to_string())
}

/// 保存済み画像（DB が参照しているファイル）の合計サイズ（バイト）を返す。
#[tauri::command]
pub fn image_storage_bytes(state: State<AppState>) -> Result<u64, String> {
    let conn = state.lock()?;
    let mut stmt = conn
        .prepare("SELECT image_path FROM clips WHERE kind = 'image' AND image_path IS NOT NULL")
        .map_err(|e| e.to_string())?;
    let paths: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    let mut total = 0u64;
    for p in paths {
        if let Ok(meta) = std::fs::metadata(p) {
            total += meta.len();
        }
    }
    Ok(total)
}

/// 画像の自動容量制限の既定値（MB）。
pub const DEFAULT_IMAGE_LIMIT_MB: u64 = 500;

/// 設定の上限（MB）を返す（未設定や不正は既定 500）。
pub fn image_limit_mb(state: &AppState) -> u64 {
    crate::settings::get_one(state, "image_limit_mb")
        .and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_IMAGE_LIMIT_MB)
}

/// 画像の合計サイズが limit_bytes を超えていたら、古い順に（ブックマークは除く）
/// 自動削除して上限以下に収める。削除した合計バイト数を返す。
pub fn enforce_image_limit(state: &AppState, limit_bytes: u64) -> Result<u64, String> {
    let conn = state.lock()?;

    // 現在の合計サイズ（DB が参照する全画像ファイル）。
    let mut total: u64 = {
        let mut stmt = conn
            .prepare("SELECT image_path FROM clips WHERE kind = 'image' AND image_path IS NOT NULL")
            .map_err(|e| e.to_string())?;
        let paths: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        paths
            .iter()
            .filter_map(|p| std::fs::metadata(p).ok().map(|m| m.len()))
            .sum()
    };
    if total <= limit_bytes {
        return Ok(0);
    }

    // ブックマークでない画像を古い順に削除候補として取得。
    let candidates: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, image_path FROM clips
                 WHERE kind = 'image' AND bookmark = 0 AND image_path IS NOT NULL
                 ORDER BY created_at ASC, id ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    let mut removed = 0u64;
    for (id, path) in candidates {
        if total <= limit_bytes {
            break;
        }
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let _ = std::fs::remove_file(&path);
        conn.execute("DELETE FROM clips WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        total = total.saturating_sub(size);
        removed = removed.saturating_add(size);
    }
    Ok(removed)
}

/// 上限(MB)を保存し、その場で適用（超過分を削除）する。
#[tauri::command]
pub fn set_image_limit(mb: u64, app: AppHandle, state: State<AppState>) -> Result<(), String> {
    crate::settings::set_value(&state, "image_limit_mb", &mb.to_string())?;
    enforce_image_limit(&state, mb.saturating_mul(1024 * 1024))?;
    let _ = app.emit("clips-changed", ());
    Ok(())
}
