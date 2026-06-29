// clipboard.rs — クリップボードの監視・保存、貼り付け（テキスト/画像）、PNG 変換。

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// クリップボード監視のポーリング間隔。
const POLL_INTERVAL_MS: u64 = 700;
/// クリップボードへ載せてから Ctrl+V を送るまでの待ち（反映待ち）。
const PASTE_DELAY_MS: u64 = 120;

/// 選択した履歴を「元アプリへ貼り付け」する。
/// クリップボードへ載せ → kopipe を隠し → 記録した入力先へフォーカスを戻し → Ctrl+V。
/// `html` があればプレーンと HTML の両方を載せる（貼り付け先がリッチを扱えれば HTML）。
#[tauri::command]
pub fn paste_clip(content: String, html: Option<String>, app: AppHandle) -> Result<(), String> {
    set_clipboard_text(&app, &content, html.as_deref())?;
    finish_paste(&app);
    Ok(())
}

/// クリップボードへ載せた後の共通処理：必要なら main を隠し、記録した入力先へフォーカスを
/// 戻して Ctrl+V を送る。ピン留め中は main を隠さず前面に残す。
fn finish_paste(app: &AppHandle) {
    use std::{thread, time::Duration};

    if super::window::should_hide_after_paste(super::window::is_pinned()) {
        if let Some(win) = app.get_webview_window(super::window::MAIN) {
            let _ = win.hide();
        }
    }
    #[cfg(windows)]
    super::win32::restore_prev_focus();

    thread::sleep(Duration::from_millis(PASTE_DELAY_MS));
    super::win32::send_ctrl_v();
}

/// メニューの「コピー」。`html` があればプレーンと HTML の両方をクリップボードへ載せる。
#[tauri::command]
pub fn copy_text(content: String, html: Option<String>, app: AppHandle) -> Result<(), String> {
    set_clipboard_text(&app, &content, html.as_deref())
}

/// プレーン（＋あれば HTML）をクリップボードへ載せる共通処理。
fn set_clipboard_text(app: &AppHandle, content: &str, html: Option<&str>) -> Result<(), String> {
    #[cfg(windows)]
    if let Some(html) = html {
        return super::win32::set_clipboard_html(content, html);
    }
    #[cfg(not(windows))]
    let _ = html;
    app.clipboard()
        .write_text(content.to_string())
        .map_err(|e| e.to_string())
}

/// 画像履歴を「元アプリへ貼り付け」する。PNG を読み込んでクリップボードへ載せ、Ctrl+V。
#[tauri::command]
pub fn paste_image(id: i64, app: AppHandle) -> Result<(), String> {
    use tauri::image::Image;

    let path = crate::image_storage::image_path(&app.state::<crate::db::AppState>(), id)?;
    let (rgba, w, h) = decode_png(&path)?;

    app.clipboard()
        .write_image(&Image::new_owned(rgba, w, h))
        .map_err(|e| e.to_string())?;

    finish_paste(&app);
    Ok(())
}

/// ファイル一覧を CF_HDROP で書き戻し、元アプリ（エクスプローラー等）へ貼り付ける。
#[tauri::command]
pub fn paste_files(paths: Vec<String>, app: AppHandle) -> Result<(), String> {
    super::win32::set_clipboard_files(&paths)?;
    finish_paste(&app);
    Ok(())
}

/// ファイル一覧を CF_HDROP でクリップボードへ載せる（貼り付けはしない）。
#[tauri::command]
pub fn copy_files(paths: Vec<String>) -> Result<(), String> {
    super::win32::set_clipboard_files(&paths)
}

/// エクスプローラーで開く。ディレクトリはそのまま開き、ファイルは親を開いて選択する。
#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::path::Path;
        use std::process::Command;

        // 実在するパスだけを対象にする（削除・移動済みの古い履歴で誤った場所を開かない）。
        let p = Path::new(&path);
        if !p.exists() {
            return Err(format!("path not found: {path}"));
        }
        // explorer は成功時も非 0 を返すことがあるので spawn のみ（終了コードは見ない）。
        let arg = if p.is_dir() {
            path.clone()
        } else {
            format!("/select,{path}")
        };
        Command::new("explorer")
            .arg(arg)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("reveal is only implemented on Windows".into())
    }
}

/// クリップボードを定期的に監視し、変化したテキスト/画像を自動で履歴に保存する。
/// 別スレッドでポーリングし、保存時はフロントへ "clips-changed" を emit してライブ更新させる。
pub(crate) fn setup_clipboard_monitor(app: &AppHandle) {
    use std::time::Duration;

    use crate::clips::record_text;
    use crate::db::AppState;

    let app = app.clone();
    std::thread::spawn(move || {
        // 起動時点の内容を「最後に見た値」として初期化（既存内容を即保存しないため）。
        let mut last_text = app.clipboard().read_text().unwrap_or_default();
        #[cfg(windows)]
        let mut last_files = super::win32::read_clipboard_files()
            .map(|f| f.join("\n"))
            .unwrap_or_default();
        let mut last_image = app
            .clipboard()
            .read_image()
            .ok()
            .map(|im| hash_bytes(im.rgba()))
            .unwrap_or_default();

        loop {
            std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));

            // ピン留め中は常駐したまま他アプリへ貼り付けるので、貼り付け先（直前のフォーカス）
            // を継続的に更新しておく。前面が自分（kopipe）のときは更新しない。
            #[cfg(windows)]
            if super::window::is_pinned() && !super::win32::foreground_is_ours() {
                super::win32::capture_prev_focus();
            }

            // まずテキストを試す。読めたらこのラウンドはテキストとして処理。
            if let Ok(text) = app.clipboard().read_text() {
                if !text.trim().is_empty() && text != last_text {
                    last_text = text.clone();
                    // 同じクリップにリッチ形式(HTML)があれば併せて保存する。
                    #[cfg(windows)]
                    let html = super::win32::read_clipboard_html();
                    #[cfg(not(windows))]
                    let html: Option<String> = None;
                    match record_text(&app.state::<AppState>(), &text, html.as_deref()) {
                        Ok(true) => {
                            let _ = app.emit("clips-changed", ());
                        }
                        Ok(false) => {}
                        Err(e) => eprintln!("record_text failed: {e}"),
                    }
                }
                continue;
            }

            // テキストが無ければファイル/フォルダ（CF_HDROP）を試す。
            #[cfg(windows)]
            {
                if let Some(files) = super::win32::read_clipboard_files() {
                    let joined = files.join("\n");
                    if !joined.is_empty() && joined != last_files {
                        last_files = joined.clone();
                        match crate::clips::record_files(&app.state::<AppState>(), &joined) {
                            Ok(true) => {
                                let _ = app.emit("clips-changed", ());
                            }
                            Ok(false) => {}
                            Err(e) => eprintln!("record_files failed: {e}"),
                        }
                    }
                    continue;
                }
            }

            // それも無ければ画像を試す。
            if let Ok(img) = app.clipboard().read_image() {
                let rgba = img.rgba();
                if rgba.is_empty() {
                    continue;
                }
                let hash = hash_bytes(rgba);
                if hash == last_image {
                    continue; // 同じ画像が載りっぱなし → 何もしない
                }
                last_image = hash.clone();
                // 保存先は設定で変えられるので、保存のたびに解決して用意する。
                let dir = crate::image_storage::image_dir(&app);
                let _ = std::fs::create_dir_all(&dir);
                match save_image_clip(&app, &dir, rgba, img.width(), img.height(), &hash) {
                    Ok(()) => {
                        let _ = app.emit("clips-changed", ());
                        // 上限超過なら古い画像を自動削除。
                        let state = app.state::<AppState>();
                        let limit = crate::image_storage::image_limit_mb(&state)
                            .saturating_mul(1024 * 1024);
                        if let Ok(removed) =
                            crate::image_storage::enforce_image_limit(&state, limit)
                        {
                            if removed > 0 {
                                let _ = app.emit("clips-changed", ());
                            }
                        }
                    }
                    Err(e) => eprintln!("save_image_clip failed: {e}"),
                }
            }
        }
    });
}

/// RGBA を PNG にエンコードして保存し、DB に画像行を追加する。
fn save_image_clip(
    app: &AppHandle,
    dir: &std::path::Path,
    rgba: &[u8],
    width: u32,
    height: u32,
    hash: &str,
) -> Result<(), String> {
    let png = encode_png(rgba, width, height)?;
    let path = dir.join(format!("{hash}.png"));
    std::fs::write(&path, &png).map_err(|e| e.to_string())?;
    let path_str = path.to_string_lossy().to_string();
    crate::clips::record_image(&app.state::<crate::db::AppState>(), &path_str, hash)?;
    Ok(())
}

/// バイト列の安定ハッシュ（重複画像の検出とファイル名に使う）。
fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// RGBA8 ピクセルを PNG バイト列にエンコードする。
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
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
fn decode_png(path: &str) -> Result<(Vec<u8>, u32, u32), String> {
    let file = std::io::BufReader::new(std::fs::File::open(path).map_err(|e| e.to_string())?);
    let mut reader = png::Decoder::new(file)
        .read_info()
        .map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "png buffer size unknown".to_string())?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    Ok((buf, info.width, info.height))
}
