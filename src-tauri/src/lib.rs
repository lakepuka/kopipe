// kopipe backend — エントリ。プラグイン・ウィンドウイベント・初期化・コマンド登録を組み立てる。
//
// Rust メモ:
// - `mod xxx;` で同階層の xxx.rs をモジュールとして取り込む（これが無いとファイルは読まれない）。
// - `crate::` はこのクレートのルート（lib.rs）起点のパス。

mod app_info;
mod clip_model;
mod clip_search;
mod clips;
pub mod db; // seed バイナリ（src/bin/seed.rs）から open_state を使うため公開。
mod image_storage;
mod settings;
mod system;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // メインウィンドウのサイズを記憶し、次回起動時に復元する（位置は center のまま）。
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(tauri_plugin_window_state::StateFlags::SIZE)
                .with_denylist(&["settings"])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        // ログイン時の自動起動（Windows はレジストリ Run キーを内部で扱う）。
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .on_window_event(|window, event| match event {
            // main / settings は X で閉じても破棄せず「隠す」（再利用してデッドロックを避ける）。
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let label = window.label();
                if label == "main" || label == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            // フォーカスが外部アプリへ移ったら UI 全体（main＋settings）を畳む（Win+V 風）。
            // 「自分の別ウィンドウ」（main↔settings やフォルダ選択ダイアログ、ドラッグ）へ移った
            // 場合は隠さない。表示直後の一過性 blur も猶予内は無視してチラつきを防ぐ。
            tauri::WindowEvent::Focused(false) => {
                #[cfg(desktop)]
                {
                    let label = window.label();
                    if (label == system::window::MAIN || label == system::window::SETTINGS)
                        && !system::win32::foreground_is_ours()
                        && !system::window::blur_suppressed()
                    {
                        // ピン留め中はメインを残すので、次の貼り付け先として
                        // いまフォアグラウンドの外部アプリを記録しておく。
                        #[cfg(windows)]
                        if label == system::window::MAIN && system::window::is_pinned() {
                            system::win32::capture_prev_focus();
                        }
                        // 設定は常に隠す。メインはピン留め中のみ残す（hide_on_blur 内で判定）。
                        system::window::hide_on_blur(window.app_handle());
                    }
                }
            }
            // メインにフォーカスが戻ったら設定ウィンドウは閉じる（裏に残さない）。
            // ※フォルダ選択ダイアログ表示ではメインは focus されないので誤爆しない。
            tauri::WindowEvent::Focused(true) =>
            {
                #[cfg(desktop)]
                if window.label() == system::window::MAIN {
                    if let Some(s) = window
                        .app_handle()
                        .get_webview_window(system::window::SETTINGS)
                    {
                        let _ = s.hide();
                    }
                }
            }
            _ => {}
        })
        // setup はアプリ起動時に一度だけ走る初期化フック。中で ? が使える。
        .setup(|app| {
            // DB を準備して共有状態に登録。
            // OS ごとのアプリ専用データフォルダ（Windows なら %APPDATA%\<識別子>）。
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?; // 無ければ作る
            let db_path = data_dir.join("kopipe.db");
            app.manage(db::open_state(&db_path)?);
            println!("kopipe db: {}", db_path.display());

            // OS 連携（デスクトップのみ）。
            #[cfg(desktop)]
            {
                system::shortcut::setup_shortcut(app.handle());
                system::window::setup_tray(app.handle())?;
                system::clipboard::setup_clipboard_monitor(app.handle());
                // 設定ウィンドウは起動時に隠した状態で 1 回だけ生成しておく。
                system::window::create_settings_window(app.handle());
                // 保存済みのピン留め設定を反映。
                system::window::init_pin(app.handle());

                // 本当の初回起動だけ、トレイ常駐ではなくメインウィンドウを表示する
                // （初期設定プロンプトをフロントが出す）。以降の起動・スタートアップでは隠したまま。
                let first_run =
                    settings::get_one(&app.state::<db::AppState>(), "autostart_prompted").is_none();
                if first_run {
                    system::window::show_main(app.handle());
                }
            }

            Ok(())
        })
        // React から invoke で呼べるコマンドを登録。
        .invoke_handler(tauri::generate_handler![
            clips::list_clips,
            clips::search_clips,
            clips::toggle_bookmark,
            clips::delete_clip,
            image_storage::image_data_url,
            clips::clear_clips,
            image_storage::image_storage_bytes,
            image_storage::image_dir_path,
            image_storage::set_image_limit,
            app_info::app_version,
            settings::get_settings,
            settings::set_setting,
            system::clipboard::paste_clip,
            system::clipboard::paste_image,
            system::clipboard::paste_files,
            system::clipboard::copy_files,
            system::clipboard::copy_text,
            system::clipboard::reveal_path,
            system::window::open_settings,
            system::window::set_pinned,
            system::shortcut::set_shortcut,
            system::shortcut::reset_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
