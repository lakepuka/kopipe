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

/// 自動起動で立ち上がったことを示す引数。Run キーに付けて登録する。
/// これが無いと「ユーザーが自分で起動した」のか「ログイン時に勝手に起動した」のかを
/// 区別できず、手動起動でもウィンドウを出せない（＝起動に失敗したように見える）。
pub const FLAG: &str = "--autostart";

/// Run キーへ FLAG を付け直したかを控えるキー（移行は 1 回だけでよい）。
const FLAG_KEY: &str = "autostart_flag";

/// 引数列に FLAG が含まれるか。判定だけを純粋な関数に切り出してテスト可能にする。
fn has_flag<I: IntoIterator<Item = String>>(args: I) -> bool {
    args.into_iter().any(|a| a == FLAG)
}

/// いまのプロセスがログイン時の自動起動で立ち上がったか。
pub fn launched_by_autostart() -> bool {
    has_flag(std::env::args())
}

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

    ensure_flag(&state, &manager);
}

/// 0.3.1 以前に登録した Run キーには FLAG が付いていない。そのままだとログイン時の起動を
/// 手動起動と誤判定してウィンドウが出てしまうので、一度だけ登録し直す（enable は
/// Run キーの値を上書きするだけなので、有効なときに呼び直せば引数が付く）。
fn ensure_flag(state: &AppState, manager: &tauri_plugin_autostart::AutoLaunchManager) {
    if settings::get_one(state, FLAG_KEY).is_some() {
        return;
    }
    // 無効なら何もしなくてよい（次に有効化されるときは FLAG 付きで登録される）。
    if manager.is_enabled().unwrap_or(false) {
        if let Err(e) = manager.enable() {
            eprintln!("autostart: 引数の付け直しに失敗しました: {e}");
            return; // 済み印を付けず、次回起動でやり直す
        }
    }
    let _ = settings::set_value(state, FLAG_KEY, "1");
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

    #[test]
    fn detects_autostart_flag_in_args() {
        let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // ログイン時の自動起動（Run キーが FLAG 付きで起動する）。
        assert!(has_flag(args(&["kopipe.exe", FLAG])));
        // 手動起動（ダブルクリックやショートカット）。
        assert!(!has_flag(args(&["kopipe.exe"])));
        // 似て非なる引数は拾わない。
        assert!(!has_flag(args(&["kopipe.exe", "--autostart-nope"])));
    }
}
