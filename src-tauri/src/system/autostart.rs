// autostart.rs — ログイン時の自動起動について「ユーザーが望んだ状態」を DB に持ち、
// 起動時に OS 側（Windows は HKCU の Run キー）と食い違っていたら直す。
//
// なぜ必要か:
// NSIS インストーラーは、バージョンアップのために上書き実行すると既定で
// 「インストールする前にアンインストールする」を選んだ状態になり、旧アンインストーラーが
// `/UPDATE` なしで走る。その経路では Run キーのエントリが削除される。設定画面の
// チェックは OS 側（プラグインの isEnabled）を見ているだけなので、再インストール後に
// 何の断りもなく自動起動だけが無効化されてしまう。初回プロンプトも
// autostart_prompted が残っているため再表示されない。
//
// そこで希望状態を settings テーブルにも控えておき、起動のたびに照合して復元する。

use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::db::AppState;
use crate::settings;

/// settings テーブルのキー（フロントの SETTING_KEYS.autostart と一致させること）。
pub const KEY: &str = "autostart";

/// 照合の結果とるべき行動。判断だけを純粋な関数に切り出してテスト可能にする。
#[derive(Debug, PartialEq, Eq)]
enum Action {
    /// 何もしない（一致している / 希望状態が false）。
    None,
    /// 希望状態が未記録なので、いまの OS 側の状態を初期値として控える。
    Record(bool),
    /// 有効にしていたのに OS 側から消えている（再インストール等）ので戻す。
    Enable,
}

fn decide(desired: Option<&str>, enabled: bool) -> Action {
    match desired {
        None => Action::Record(enabled),
        Some("true") if !enabled => Action::Enable,
        // 希望が false なのに OS 側が有効な場合は触らない。kopipe 以外（タスクマネージャー等）
        // で入れた設定を勝手に取り消さないため。
        _ => Action::None,
    }
}

/// 起動時に 1 回だけ呼ぶ。失敗しても致命的ではないのでログだけ出して続行する。
pub fn restore(app: &AppHandle) {
    let state = app.state::<AppState>();
    let manager = app.autolaunch();
    let enabled = match manager.is_enabled() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("autostart: 状態を取得できませんでした: {e}");
            return;
        }
    };

    match decide(settings::get_one(&state, KEY).as_deref(), enabled) {
        Action::None => {}
        Action::Record(v) => {
            let _ = settings::set_value(&state, KEY, if v { "true" } else { "false" });
        }
        Action::Enable => match manager.enable() {
            Ok(()) => println!("autostart: 自動起動の設定が消えていたので復元しました"),
            Err(e) => eprintln!("autostart: 復元に失敗しました: {e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decides_by_desired_state_and_os_state() {
        // 未記録（旧バージョンからの引き継ぎ）→ いまの状態を控えるだけ。
        assert_eq!(decide(None, true), Action::Record(true));
        assert_eq!(decide(None, false), Action::Record(false));
        // 希望どおりなら何もしない。
        assert_eq!(decide(Some("true"), true), Action::None);
        assert_eq!(decide(Some("false"), false), Action::None);
        // 有効にしていたのに OS 側から消えた（再インストール）→ 復元する。
        assert_eq!(decide(Some("true"), false), Action::Enable);
        // 希望が false なら OS 側が有効でも触らない。
        assert_eq!(decide(Some("false"), true), Action::None);
    }
}
