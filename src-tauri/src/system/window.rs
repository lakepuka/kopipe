// window.rs — メイン/設定ウィンドウの表示制御とシステムトレイ。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

/// ウィンドウのラベル（タイポ防止のため定数化）。
pub(crate) const MAIN: &str = "main";
pub(crate) const SETTINGS: &str = "settings";

/// トレイアイコンの id。言語切替時にメニューを差し替えるため固定 id で参照する。
const TRAY_ID: &str = "main-tray";

// ピン留め（最前面固定）の状態。on のときはフォーカスを失っても隠さない。
static PINNED: AtomicBool = AtomicBool::new(false);

/// ピン留め中か（フォーカス喪失時の自動非表示を抑止するために参照）。
pub(crate) fn is_pinned() -> bool {
    PINNED.load(Ordering::Relaxed)
}

/// ピン留めを適用（最前面固定の ON/OFF と状態保持）。
/// main と settings を同じ最前面帯に乗せる（設定は後から前面化するので main の上に出る）。
fn apply_pin(app: &AppHandle, pinned: bool) {
    PINNED.store(pinned, Ordering::Relaxed);
    for label in [MAIN, SETTINGS] {
        if let Some(win) = app.get_webview_window(label) {
            let _ = win.set_always_on_top(pinned);
        }
    }
}

/// 起動時に保存済みのピン設定を反映する。
pub(crate) fn init_pin(app: &AppHandle) {
    let pinned = crate::settings::get_one(&app.state::<crate::db::AppState>(), "pinned")
        .map(|v| v == "true")
        .unwrap_or(false);
    apply_pin(app, pinned);
}

/// 設定ウィンドウからピン留めを切り替える。
#[tauri::command]
pub fn set_pinned(
    pinned: bool,
    app: AppHandle,
    state: tauri::State<crate::db::AppState>,
) -> Result<(), String> {
    crate::settings::set_value(&state, "pinned", if pinned { "true" } else { "false" })?;
    apply_pin(&app, pinned);
    // 主画面がピン状態を取り直せるよう通知（アクティブ化クリックの抑止切替に使う）。
    let _ = app.emit("settings-changed", ());
    Ok(())
}

// プログラムからウィンドウを表示した時刻。表示直後の一過性 blur（Windows が一旦
// フォーカスを戻すことがある）で隠してしまわないための猶予判定に使う。
static SHOWN_AT: Mutex<Option<Instant>> = Mutex::new(None);

/// 「いま表示した」と記録する（直後の blur を無視させる）。
fn mark_shown() {
    *SHOWN_AT.lock().unwrap() = Some(Instant::now());
}

/// 表示直後、この時間内の blur は無視する（Windows の一過性フォーカス戻り対策）。
const SHOW_BLUR_GRACE_MS: u64 = 250;

/// 表示直後の猶予内かどうか（この間の blur では隠さない）。
pub(crate) fn blur_suppressed() -> bool {
    SHOWN_AT
        .lock()
        .unwrap()
        .map(|t| t.elapsed() < Duration::from_millis(SHOW_BLUR_GRACE_MS))
        .unwrap_or(false)
}

/// フォーカスが外部へ移ったとき「(main を隠すか, settings を隠すか)」を決める純関数。
/// 設定は常に閉じる。メインはピン留め中は残す。（テスト対象）
pub(crate) fn should_hide_on_blur(pinned: bool) -> (bool, bool) {
    (!pinned, true)
}

/// ペースト後にメインを隠すか。ピン留め中は隠さない。（テスト対象）
pub(crate) fn should_hide_after_paste(pinned: bool) -> bool {
    !pinned
}

/// フォーカスが外部アプリへ移ったときの非表示。
/// 設定ウィンドウは常に隠す。メインはピン留め中だけ隠さず最前面に残す。
pub(crate) fn hide_on_blur(app: &AppHandle) {
    let (hide_main, hide_settings) = should_hide_on_blur(is_pinned());
    if hide_settings {
        if let Some(w) = app.get_webview_window(SETTINGS) {
            let _ = w.hide();
        }
    }
    if hide_main {
        if let Some(w) = app.get_webview_window(MAIN) {
            let _ = w.hide();
        }
    }
}

/// "main" ウィンドウを表示してフォーカスする（初回起動時の表示などに使う）。
pub(crate) fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN) {
        let _ = win.show();
        let _ = win.set_focus();
        mark_shown();
    }
}

/// "main" ウィンドウの表示/非表示をトグルする共通処理。
/// ショートカット・トレイ左クリック・2回押しフックから使う。
pub(crate) fn toggle_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN) {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            // 表示してフォーカスを奪う直前に、元アプリの入力フォーカスを記録しておく。
            #[cfg(windows)]
            super::win32::capture_prev_focus();
            let _ = win.show();
            let _ = win.set_focus();
            mark_shown();
        }
    }
}

/// 設定ウィンドウを「隠した状態で」生成する。起動時(setup)に 1 回だけ呼ぶ。
/// コマンドのスレッドから build() するとデッドロックしやすいので、メインスレッドの setup で作る。
pub(crate) fn create_settings_window(app: &AppHandle) {
    if app.get_webview_window(SETTINGS).is_some() {
        return;
    }
    // 同じ index.html を ?view=settings 付きで読み込み、フロントはクエリで画面を出し分ける。
    let res = tauri::WebviewWindowBuilder::new(
        app,
        SETTINGS,
        tauri::WebviewUrl::App("index.html?view=settings".into()),
    )
    .title("kopipe 設定")
    .inner_size(440.0, 380.0)
    .decorations(false)
    .skip_taskbar(true)
    .visible(false)
    .build();
    if let Err(e) = res {
        eprintln!("failed to create settings window: {e}");
    }
}

/// 既存の設定ウィンドウを表示する。
/// メインが表示中ならその中央へ重ね、そうでなければ画面中央に出す
/// （メインが非表示だと位置が確定せず画面外に出ることがあるため）。
fn show_settings(app: &AppHandle) {
    let Some(w) = app.get_webview_window(SETTINGS) else {
        eprintln!("settings window not created");
        return;
    };

    let positioned = app
        .get_webview_window(MAIN)
        .filter(|m| m.is_visible().unwrap_or(false))
        .and_then(|m| {
            let (mpos, msize, ssize) = (
                m.outer_position().ok()?,
                m.outer_size().ok()?,
                w.outer_size().ok()?,
            );
            let x = mpos.x + (msize.width as i32 - ssize.width as i32) / 2;
            let y = mpos.y + (msize.height as i32 - ssize.height as i32) / 2;
            w.set_position(tauri::PhysicalPosition::new(x, y)).ok()
        })
        .is_some();
    if !positioned {
        let _ = w.center();
    }

    // ピン留め中はメインが最前面固定なので、設定も同帯に乗せて上に出す。
    let _ = w.set_always_on_top(is_pinned());
    let _ = w.show();
    let _ = w.set_focus();
    mark_shown();
}

/// 設定ウィンドウを開くコマンド（主画面のギアボタンから呼ぶ）。
#[tauri::command]
pub fn open_settings(app: AppHandle) {
    show_settings(&app);
}

/// トレイメニューの表示文言（フロントの i18n と対応。ここは小さいので Rust 側に持つ）。
/// 返り値は (表示, 設定, 終了)。未知の言語は英語にフォールバックする。
fn tray_labels(lang: &str) -> (&'static str, &'static str, &'static str) {
    match lang {
        "ja" => ("表示", "設定", "終了"),
        _ => ("Show", "Settings", "Quit"),
    }
}

/// 保存済みの言語設定を読む（未設定は既定の英語）。
fn current_lang(app: &AppHandle) -> String {
    crate::settings::get_one(&app.state::<crate::db::AppState>(), "lang")
        .unwrap_or_else(|| "en".to_string())
}

/// 指定言語のトレイメニューを組み立てる。id は固定なので on_menu_event はそのまま使える。
fn build_tray_menu(app: &AppHandle, lang: &str) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem};

    let (show, settings, quit) = tray_labels(lang);
    let show_i = MenuItem::with_id(app, "show", show, true, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, "settings", settings, true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    Menu::with_items(app, &[&show_i, &settings_i, &quit_i])
}

/// 言語切替時にトレイメニューの文言を差し替える（settings の set_setting から呼ぶ）。
pub(crate) fn apply_tray_lang(app: &AppHandle, lang: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        match build_tray_menu(app, lang) {
            Ok(menu) => {
                let _ = tray.set_menu(Some(menu));
            }
            Err(e) => eprintln!("failed to rebuild tray menu: {e}"),
        }
    }
}

/// システムトレイを構築する。左クリックでトグル、右クリックメニューで表示/設定/終了。
pub(crate) fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let menu = build_tray_menu(app, &current_lang(app))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("kopipe")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(win) = app.get_webview_window(MAIN) {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "settings" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{should_hide_after_paste, should_hide_on_blur};

    #[test]
    fn blur_hides_settings_always_main_only_when_unpinned() {
        // (hide_main, hide_settings)
        assert_eq!(should_hide_on_blur(false), (true, true));
        assert_eq!(should_hide_on_blur(true), (false, true));
    }

    #[test]
    fn paste_hides_main_only_when_unpinned() {
        assert!(should_hide_after_paste(false));
        assert!(!should_hide_after_paste(true));
    }
}
