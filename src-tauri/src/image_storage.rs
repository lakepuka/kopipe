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

/// 指定 id の画像ファイルパスとハッシュを返す（画像行のみ）。
/// 「履歴から貼り付け」でその場で record_image を呼び直す（＝先頭へ昇格）ために使う。
pub fn image_path_and_hash(state: &AppState, id: i64) -> Result<(String, String), String> {
    let conn = state.lock()?;
    conn.query_row(
        "SELECT image_path, hash FROM clips WHERE id = ?1 AND kind = 'image'",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|e| e.to_string())
}

/// PNG バイト列をデータURL（data:image/png;base64,...）にする。
fn png_data_url(bytes: &[u8]) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:image/png;base64,{b64}")
}

/// 画像を原寸のデータURLにして返す。プレビュー（拡大表示）用。
/// async にしてワーカースレッドで実行する。同期コマンドは UI スレッドで走るため、
/// 数十 MB の PNG を読んで base64 にする間、ウィンドウ全体が固まってしまう。
#[tauri::command(async)]
pub fn image_data_url(id: i64, state: State<'_, AppState>) -> Result<String, String> {
    let path = image_path(&state, id)?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(png_data_url(&bytes))
}

/// 一覧のサムネイル枠（CSS の max-height 80px）を高 DPI でも粗く見せない大きさ。
const THUMB_MAX_W: u32 = 1080;
const THUMB_MAX_H: u32 = 160;

/// サムネイルのキャッシュ（キーは PNG のパス＝内容ハッシュ）。上限を超えたら丸ごと捨てる。
/// メモリ上だけに持ち、ディスクには書かない（消した画像の縮小版が残らないように）。
const THUMB_CACHE_MAX_BYTES: usize = 32 * 1024 * 1024;
static THUMB_CACHE: std::sync::Mutex<Option<(std::collections::HashMap<String, String>, usize)>> =
    std::sync::Mutex::new(None);

/// 一覧用のサムネイルをデータURLで返す。
/// 以前は一覧の各行が原寸 PNG を丸ごと base64 で受け取っており、スクリーンショットだと
/// 1 行で数 MB〜数十 MB を IPC で運び、WebView が原寸のまま展開していた。
/// 縮小はワーカースレッドで行い（async）、DB のロックは パス取得の間しか持たない。
#[tauri::command(async)]
pub fn image_thumb_data_url(id: i64, state: State<'_, AppState>) -> Result<String, String> {
    let path = image_path(&state, id)?;
    if let Some(hit) = THUMB_CACHE
        .lock()
        .ok()
        .and_then(|c| c.as_ref().and_then(|(m, _)| m.get(&path).cloned()))
    {
        return Ok(hit);
    }

    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let (rgba, w, h) = decode_png_bytes(&bytes)?;
    let (tw, th) = thumb_size(w, h);
    let url = if (tw, th) == (w, h) {
        // 既に枠に収まる小さな画像は再エンコードせずそのまま返す。
        png_data_url(&bytes)
    } else {
        png_data_url(&encode_png(&downscale_rgba(&rgba, w, h, tw, th), tw, th)?)
    };

    if let Ok(mut cache) = THUMB_CACHE.lock() {
        let (map, total) = cache.get_or_insert_with(Default::default);
        if *total + url.len() > THUMB_CACHE_MAX_BYTES {
            map.clear();
            *total = 0;
        }
        *total += url.len();
        map.insert(path, url.clone());
    }
    Ok(url)
}

/// 縦横比を保って THUMB_MAX_W × THUMB_MAX_H に収まる大きさ（拡大はしない）。（テスト対象）
fn thumb_size(w: u32, h: u32) -> (u32, u32) {
    let s = (THUMB_MAX_W as f64 / w as f64)
        .min(THUMB_MAX_H as f64 / h as f64)
        .min(1.0);
    (
        ((w as f64 * s).round() as u32).max(1),
        ((h as f64 * s).round() as u32).max(1),
    )
}

/// RGBA8 を面積平均で縮小する（外部クレートを増やさないため自前）。
/// 色は alpha で重み付けして平均する。そのまま平均すると、透明部分の（見えない）色が
/// 縁ににじむため。（テスト対象）
fn downscale_rgba(src: &[u8], w: u32, h: u32, dw: u32, dh: u32) -> Vec<u8> {
    let (w, h, dw, dh) = (w as usize, h as usize, dw as usize, dh as usize);
    let mut out = vec![0u8; dw * dh * 4];
    for dy in 0..dh {
        let y0 = dy * h / dh;
        let y1 = ((dy + 1) * h / dh).max(y0 + 1);
        for dx in 0..dw {
            let x0 = dx * w / dw;
            let x1 = ((dx + 1) * w / dw).max(x0 + 1);
            let (mut r, mut g, mut b, mut a) = (0u64, 0u64, 0u64, 0u64);
            for y in y0..y1 {
                for px in src[(y * w + x0) * 4..(y * w + x1) * 4].chunks_exact(4) {
                    let pa = px[3] as u64;
                    r += px[0] as u64 * pa;
                    g += px[1] as u64 * pa;
                    b += px[2] as u64 * pa;
                    a += pa;
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u64;
            let o = (dy * dw + dx) * 4;
            if a > 0 {
                out[o] = ((r + a / 2) / a) as u8;
                out[o + 1] = ((g + a / 2) / a) as u8;
                out[o + 2] = ((b + a / 2) / a) as u8;
            }
            out[o + 3] = ((a + n / 2) / n) as u8;
        }
    }
    out
}

/// RGBA8 ピクセルを PNG バイト列にエンコードする。
pub(crate) fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut buf, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    Ok(buf)
}

/// 保存した PNG を RGBA8 にデコードする（貼り戻し用）。kopipe が書いた PNG は常に RGBA8。
pub(crate) fn decode_png(path: &str) -> Result<(Vec<u8>, u32, u32), String> {
    decode_png_bytes(&std::fs::read(path).map_err(|e| e.to_string())?)
}

fn decode_png_bytes(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .map_err(|e| e.to_string())?;
    // RGBA8 以外を RGBA8 として扱うと崩れた画像になるので、はっきり失敗させる。
    if reader.output_color_type() != (png::ColorType::Rgba, png::BitDepth::Eight) {
        return Err("unsupported png format (expected RGBA8)".into());
    }
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "png buffer size unknown".to_string())?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    Ok((buf, info.width, info.height))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_size_fits_box_keeps_ratio_and_never_upscales() {
        assert_eq!(thumb_size(100, 50), (100, 50)); // 収まる → そのまま
        assert_eq!(thumb_size(3840, 2160), (284, 160)); // 4K 横長 → 高さで決まる
        assert_eq!(thumb_size(20000, 100), (1080, 5)); // 極端に横長 → 幅で決まる
        assert_eq!(thumb_size(1, 100000), (1, 160)); // 1px 未満に潰さない
    }

    #[test]
    fn downscale_averages_and_weights_by_alpha() {
        // 2×2 → 1×1。不透明の赤 2 つと、透明（色は緑だが見えない）2 つ。
        let src = [
            255, 0, 0, 255, 0, 255, 0, 0, //
            255, 0, 0, 255, 0, 255, 0, 0,
        ];
        let out = downscale_rgba(&src, 2, 2, 1, 1);
        // 透明部分の緑はにじまず赤のまま、不透明度は半分。
        assert_eq!(out, vec![255, 0, 0, 128]);

        // 4×2 → 2×1 で左右の領域が混ざらない。
        let mut src = Vec::new();
        for _ in 0..2 {
            src.extend([
                10, 10, 10, 255, 10, 10, 10, 255, 200, 200, 200, 255, 200, 200, 200, 255,
            ]);
        }
        assert_eq!(
            downscale_rgba(&src, 4, 2, 2, 1),
            vec![10, 10, 10, 255, 200, 200, 200, 255]
        );
    }

    #[test]
    fn png_roundtrip_and_rejects_non_rgba8() {
        let rgba: Vec<u8> = (0..(3 * 2 * 4)).map(|i| i as u8).collect();
        let png = encode_png(&rgba, 3, 2).unwrap();
        assert_eq!(decode_png_bytes(&png).unwrap(), (rgba, 3, 2));

        // RGB（alpha なし）の PNG は RGBA8 として誤読せずエラーにする。
        let mut rgb_png = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut rgb_png, 1, 1);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header()
                .unwrap()
                .write_image_data(&[1, 2, 3])
                .unwrap();
        }
        assert!(decode_png_bytes(&rgb_png).is_err());
    }
}
