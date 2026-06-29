// app_info.rs — アプリ自身のメタ情報を返す小さなコマンド。

use tauri::AppHandle;

/// アプリのバージョン（tauri.conf.json の version）を返す。
#[tauri::command]
pub fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}
