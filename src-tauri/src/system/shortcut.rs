// shortcut.rs — 呼び出しトリガーの登録（キーコンボ / 修飾キー2回）と設定リセット。

use tauri::{AppHandle, Emitter, Manager};

/// 既定の呼び出しトリガー。"double:Shift" = Shift の2回連続。
const DEFAULT_SHORTCUT: &str = "double:Shift";

/// 呼び出しトリガーを登録する。
/// - "double:Shift" / "double:Control" … 修飾キー2回（低レベルフックで検出）
/// - それ以外（"Control+Shift+KeyV" 等） … キーコンボ（global-shortcut）
fn register_shortcut(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let gs = app.global_shortcut();
    let _ = gs.unregister_all(); // 以前のコンボ登録を解除

    if let Some(modifier) = accelerator.strip_prefix("double:") {
        let code = match modifier {
            "Shift" => 1,
            "Control" => 2,
            _ => return Err(format!("invalid trigger: {accelerator}")),
        };
        super::win32::set_target_mod(code); // フックが拾うように
        return Ok(());
    }

    // コンボモード: 2回検出を無効化し、グローバルショートカットを登録。
    super::win32::set_target_mod(0);
    gs.on_shortcut(accelerator, |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            super::window::toggle_window(app);
        }
    })
    .map_err(|e| e.to_string())
}

/// 起動時: 低レベルキーボードフックを仕込み、保存済み（無ければ既定）のトリガーを登録する。
pub(crate) fn setup_shortcut(app: &AppHandle) {
    use crate::db::AppState;

    #[cfg(windows)]
    super::win32::install_keyboard_hook(app);

    let accel = crate::settings::get_one(&app.state::<AppState>(), "shortcut")
        .unwrap_or_else(|| DEFAULT_SHORTCUT.to_string());
    if register_shortcut(app, &accel).is_err() {
        let _ = register_shortcut(app, DEFAULT_SHORTCUT);
    }
}

/// ショートカットを変更するコマンド。登録（＝検証）してから保存する。
#[tauri::command]
pub fn set_shortcut(accelerator: String, app: AppHandle) -> Result<(), String> {
    use crate::db::AppState;

    register_shortcut(&app, &accelerator)?; // 不正ならここで Err
    crate::settings::set_value(&app.state::<AppState>(), "shortcut", &accelerator)?;
    let _ = app.emit("settings-changed", ());
    Ok(())
}

/// 設定を初期状態に戻す（全設定を消し、トリガーを既定で登録し直す）。
#[tauri::command]
pub fn reset_settings(app: AppHandle) -> Result<(), String> {
    use crate::db::AppState;

    crate::settings::clear(&app.state::<AppState>())?;
    let _ = register_shortcut(&app, DEFAULT_SHORTCUT);
    let _ = app.emit("settings-changed", ());
    Ok(())
}
