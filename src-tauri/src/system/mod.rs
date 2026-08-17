// system — OS 連携。デスクトップ専用。
//
// - win32:    Windows 低レベル（フォーカス・Ctrl+V・修飾キー2回フック）
// - window:   メイン/設定ウィンドウの表示制御・トレイ
// - shortcut: 呼び出しトリガー登録・設定リセット
// - clipboard: クリップボード監視/保存・貼り付け・PNG 変換
// - autostart: ログイン時の自動起動の希望状態を保持し、起動時に OS 側へ復元
#![cfg(desktop)]

pub mod autostart;
pub mod clipboard;
pub mod shortcut;
pub mod win32;
pub mod window;
