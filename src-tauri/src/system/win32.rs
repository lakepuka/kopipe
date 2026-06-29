// win32.rs — Windows 低レベル処理。
// フォーカスの記録/復元、Ctrl+V の擬似入力、修飾キー2回連続の検出（低レベルフック）。
// 非 Windows ではフォーカス系・送出はスタブ。

use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

// 修飾キー2回検出のターゲット。0=無効（コンボモード）, 1=Shift, 2=Ctrl。
static TARGET_MOD: AtomicU8 = AtomicU8::new(0);

/// 2回検出のターゲットを設定する（0=無効=コンボモード）。
pub(crate) fn set_target_mod(code: u8) {
    TARGET_MOD.store(code, Relaxed);
}

/// Ctrl+V を OS に擬似入力する（Windows: Win32 SendInput）。
/// enigo 等の外部クレートを避け、公式 windows クレートだけで実装。
#[cfg(windows)]
pub(crate) fn send_ctrl_v() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VIRTUAL_KEY, VK_CONTROL, VK_V,
    };

    fn ev(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    // Ctrl 押下 → V 押下 → V 離上 → Ctrl 離上。
    let inputs = [
        ev(VK_CONTROL, KEYBD_EVENT_FLAGS(0)),
        ev(VK_V, KEYBD_EVENT_FLAGS(0)),
        ev(VK_V, KEYEVENTF_KEYUP),
        ev(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(not(windows))]
pub(crate) fn send_ctrl_v() {
    eprintln!("paste key simulation is only implemented on Windows");
}

// 呼び出し直前にフォーカスがあった (前面ウィンドウ, 子コントロール) の HWND を isize で保持。
#[cfg(windows)]
static PREV_FOCUS: std::sync::Mutex<Option<(isize, isize)>> = std::sync::Mutex::new(None);

/// 呼び出し直前にフォーカスを持っていた前面ウィンドウと入力コントロールを記録する。
#[cfg(windows)]
pub(crate) fn capture_prev_focus() {
    use std::mem::size_of;

    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
    };

    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            *PREV_FOCUS.lock().unwrap() = None;
            return;
        }
        let tid = GetWindowThreadProcessId(fg, None);
        let mut gti = GUITHREADINFO {
            cbSize: size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        let focus = if GetGUIThreadInfo(tid, &mut gti).is_ok() && !gti.hwndFocus.0.is_null() {
            gti.hwndFocus
        } else {
            fg
        };
        *PREV_FOCUS.lock().unwrap() = Some((fg.0 as isize, focus.0 as isize));
    }
}

/// 記録しておいたコントロールへフォーカスを戻す（貼り付け先を正確に復元する）。
#[cfg(windows)]
pub(crate) fn restore_prev_focus() {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowThreadProcessId, SetForegroundWindow};

    let target = *PREV_FOCUS.lock().unwrap();
    if let Some((fg, focus)) = target {
        unsafe {
            let fg = HWND(fg as *mut core::ffi::c_void);
            let focus = HWND(focus as *mut core::ffi::c_void);
            let _ = SetForegroundWindow(fg);
            let our_tid = GetCurrentThreadId();
            let target_tid = GetWindowThreadProcessId(fg, None);
            let _ = AttachThreadInput(our_tid, target_tid, true);
            let _ = SetFocus(Some(focus));
            let _ = AttachThreadInput(our_tid, target_tid, false);
        }
    }
}

/// いま前面にあるウィンドウが kopipe 自身（同じプロセス）かを判定する。
#[cfg(windows)]
pub(crate) fn foreground_is_ours() -> bool {
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}

#[cfg(not(windows))]
pub(crate) fn foreground_is_ours() -> bool {
    false
}

/// クリップボードにファイル/フォルダ（CF_HDROP）があればそのパス一覧を返す。
/// プラグインの read_text/read_image では取れないので Win32 で直接読む。
#[cfg(windows)]
pub(crate) fn read_clipboard_files() -> Option<Vec<String>> {
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

    const CF_HDROP: u32 = 15;
    unsafe {
        if IsClipboardFormatAvailable(CF_HDROP).is_err() {
            return None;
        }
        if OpenClipboard(None).is_err() {
            return None;
        }
        // OpenClipboard 後は必ず Close するため、処理をクロージャに包んで結果だけ受ける。
        let result = (|| {
            let handle = GetClipboardData(CF_HDROP).ok()?;
            let hdrop = HDROP(handle.0);
            let count = DragQueryFileW(hdrop, u32::MAX, None); // ファイル数
            if count == 0 {
                return None;
            }
            let mut paths = Vec::with_capacity(count as usize);
            for i in 0..count {
                let len = DragQueryFileW(hdrop, i, None); // null を除く文字数
                if len == 0 {
                    continue;
                }
                let mut buf = vec![0u16; len as usize + 1];
                let copied = DragQueryFileW(hdrop, i, Some(&mut buf));
                if copied > 0 {
                    paths.push(String::from_utf16_lossy(&buf[..copied as usize]));
                }
            }
            (!paths.is_empty()).then_some(paths)
        })();
        let _ = CloseClipboard();
        result
    }
}

#[cfg(not(windows))]
pub(crate) fn read_clipboard_files() -> Option<Vec<String>> {
    None
}

/// ファイル一覧をクリップボードへ CF_HDROP として書き込む（エクスプローラー等が貼り付け可能）。
#[cfg(windows)]
pub(crate) fn set_clipboard_files(paths: &[String]) -> Result<(), String> {
    use std::mem::size_of;

    use windows::Win32::Foundation::{HANDLE, POINT};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::UI::Shell::DROPFILES;

    const CF_HDROP: u32 = 15;
    if paths.is_empty() {
        return Err("no files".into());
    }

    // ワイド文字列リスト（各パスを null 終端 + 末尾に追加の null＝ダブル null 終端）。
    let mut wide: Vec<u16> = Vec::new();
    for p in paths {
        wide.extend(p.encode_utf16());
        wide.push(0);
    }
    wide.push(0);

    let header = size_of::<DROPFILES>();
    let total = header + wide.len() * size_of::<u16>();

    unsafe {
        let hmem = GlobalAlloc(GMEM_MOVEABLE, total).map_err(|e| e.to_string())?;
        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            return Err("GlobalLock failed".into());
        }
        // DROPFILES ヘッダ。pFiles はファイルリストの開始オフセット、fWide はワイド文字。
        let df = ptr as *mut DROPFILES;
        (*df).pFiles = header as u32;
        (*df).pt = POINT { x: 0, y: 0 };
        (*df).fNC = false.into();
        (*df).fWide = true.into();
        // ヘッダ直後にパスリストをコピー。
        let dst = (ptr as *mut u8).add(header) as *mut u16;
        std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
        let _ = GlobalUnlock(hmem);

        if OpenClipboard(None).is_err() {
            return Err("OpenClipboard failed".into());
        }
        let _ = EmptyClipboard();
        // 成功すると確保したメモリの所有権は OS に移る（自前で解放しない）。
        let res = SetClipboardData(CF_HDROP, Some(HANDLE(hmem.0)));
        let _ = CloseClipboard();
        res.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn set_clipboard_files(_paths: &[String]) -> Result<(), String> {
    Err("file paste is only implemented on Windows".into())
}

// ---- HTML（リッチ形式）クリップボード ----
// Windows の "HTML Format" は、本文の前に StartHTML/EndHTML/StartFragment/EndFragment
// というバイトオフセットを書いたヘッダーが必要な独特の形式。DB には素の HTML 断片だけを
// 保存し、貼り付け時にこのヘッダーを組み立てる。

/// クリップボードに HTML 形式があれば、その本文（フラグメント）を返す。
#[cfg(windows)]
pub(crate) fn read_clipboard_html() -> Option<String> {
    use windows::core::w;
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
        RegisterClipboardFormatW,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    unsafe {
        let cf = RegisterClipboardFormatW(w!("HTML Format"));
        if cf == 0 || IsClipboardFormatAvailable(cf).is_err() {
            return None;
        }
        if OpenClipboard(None).is_err() {
            return None;
        }
        let result = (|| {
            let handle = GetClipboardData(cf).ok()?;
            let hglobal = HGLOBAL(handle.0);
            let ptr = GlobalLock(hglobal) as *const u8;
            if ptr.is_null() {
                return None;
            }
            let size = GlobalSize(hglobal);
            let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
            let _ = GlobalUnlock(hglobal);
            extract_html_fragment(&bytes)
        })();
        let _ = CloseClipboard();
        result
    }
}

/// CF_HTML データから StartFragment/EndFragment のオフセットで本文を切り出す。
#[cfg(windows)]
fn extract_html_fragment(bytes: &[u8]) -> Option<String> {
    // ヘッダーは先頭の ASCII 部分にある。
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
    let start = parse_cf_html_offset(&head, "StartFragment:")?;
    let end = parse_cf_html_offset(&head, "EndFragment:")?;
    if start > end || end > bytes.len() {
        return None;
    }
    let frag = String::from_utf8_lossy(&bytes[start..end])
        .trim()
        .to_string();
    (!frag.is_empty()).then_some(frag)
}

/// `key`（例 "StartFragment:"）に続く 10 進数のバイトオフセットを読む。
#[cfg(windows)]
fn parse_cf_html_offset(head: &str, key: &str) -> Option<usize> {
    let i = head.find(key)? + key.len();
    head[i..]
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}

/// プレーン(CF_UNICODETEXT)と HTML(CF_HTML)の両方を一度にクリップボードへ載せる。
/// 貼り付け先がリッチを扱えれば HTML を、ダメならプレーンを採用する。
#[cfg(windows)]
pub(crate) fn set_clipboard_html(plain: &str, html: &str) -> Result<(), String> {
    use windows::core::w;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };

    const CF_UNICODETEXT: u32 = 13;

    let cf_html = unsafe { RegisterClipboardFormatW(w!("HTML Format")) };
    if cf_html == 0 {
        return Err("RegisterClipboardFormatW failed".into());
    }

    // プレーンは UTF-16 + null 終端。
    let mut wide: Vec<u16> = plain.encode_utf16().collect();
    wide.push(0);
    let text_bytes =
        unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };

    // HTML は CF_HTML 形式（オフセットヘッダー付き）の UTF-8 + null 終端。
    let mut html_bytes = build_cf_html(html).into_bytes();
    html_bytes.push(0);

    unsafe {
        let h_text = global_from_bytes(text_bytes)?;
        let h_html = global_from_bytes(&html_bytes)?;
        if OpenClipboard(None).is_err() {
            return Err("OpenClipboard failed".into());
        }
        let _ = EmptyClipboard();
        // 成功すると確保メモリの所有権は OS へ移る。
        let r1 = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(h_text.0)));
        let r2 = SetClipboardData(cf_html, Some(HANDLE(h_html.0)));
        let _ = CloseClipboard();
        r1.map_err(|e| e.to_string())?;
        r2.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// バイト列を GMEM_MOVEABLE で確保してコピーし、HGLOBAL を返す（SetClipboardData 用）。
#[cfg(windows)]
unsafe fn global_from_bytes(bytes: &[u8]) -> Result<windows::Win32::Foundation::HGLOBAL, String> {
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|e| e.to_string())?;
    let p = GlobalLock(h);
    if p.is_null() {
        return Err("GlobalLock failed".into());
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len());
    let _ = GlobalUnlock(h);
    Ok(h)
}

/// 素の HTML 断片を CF_HTML 形式（オフセットヘッダー付き）に組み立てる。
#[cfg(windows)]
fn build_cf_html(fragment: &str) -> String {
    let pre = "<html><body><!--StartFragment-->";
    let post = "<!--EndFragment--></body></html>";
    // 数値を 10 桁ゼロ詰めにしてヘッダー長を固定し、オフセットを 1 回で確定させる。
    let header = |sh: usize, eh: usize, sf: usize, ef: usize| {
        format!(
            "Version:0.9\r\nStartHTML:{sh:010}\r\nEndHTML:{eh:010}\r\nStartFragment:{sf:010}\r\nEndFragment:{ef:010}\r\n"
        )
    };
    let header_len = header(0, 0, 0, 0).len();
    let start_html = header_len;
    let start_fragment = header_len + pre.len();
    let end_fragment = start_fragment + fragment.len();
    let end_html = end_fragment + post.len();
    format!(
        "{}{pre}{fragment}{post}",
        header(start_html, end_html, start_fragment, end_fragment)
    )
}

#[cfg(not(windows))]
pub(crate) fn read_clipboard_html() -> Option<String> {
    None
}

#[cfg(not(windows))]
pub(crate) fn set_clipboard_html(_plain: &str, _html: &str) -> Result<(), String> {
    Err("html clipboard is only implemented on Windows".into())
}

// ---- 修飾キー2回連続の検出（低レベルキーボードフック） ----

#[cfg(windows)]
static HOOK_APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();
#[cfg(windows)]
static MOD_DOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(windows)]
static POLLUTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(windows)]
static LAST_TAP: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

/// 低レベルキーボードフックを別スレッドで仕込み、メッセージループで生かし続ける。
#[cfg(windows)]
pub(crate) fn install_keyboard_hook(app: &tauri::AppHandle) {
    let _ = HOOK_APP.set(app.clone());
    std::thread::spawn(|| unsafe {
        use windows::Win32::Foundation::HINSTANCE;
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, MSG, WH_KEYBOARD_LL,
        };

        let Ok(hmod) = GetModuleHandleW(None) else {
            return;
        };
        if SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_proc), Some(HINSTANCE(hmod.0)), 0).is_err() {
            eprintln!("failed to install keyboard hook");
            return;
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

#[cfg(windows)]
unsafe extern "system" fn ll_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{CallNextHookEx, KBDLLHOOKSTRUCT};

    if code >= 0 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        handle_key(wparam.0 as u32, kb.vkCode);
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// 修飾キーの2回タップとみなす最大間隔。
#[cfg(windows)]
const DOUBLE_TAP_MS: u64 = 400;

/// 対象修飾キーの「2回連続タップ」を検出したらトグルする。
#[cfg(windows)]
fn handle_key(msg: u32, vk: u32) {
    use std::time::{Duration, Instant};

    use windows::Win32::UI::Input::KeyboardAndMouse::{
        VK_CONTROL, VK_LCONTROL, VK_LSHIFT, VK_RCONTROL, VK_RSHIFT, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    let target = TARGET_MOD.load(Relaxed);
    if target == 0 {
        return; // コンボモードなので何もしない
    }
    let is_target = match target {
        1 => vk == VK_LSHIFT.0 as u32 || vk == VK_RSHIFT.0 as u32 || vk == VK_SHIFT.0 as u32,
        2 => vk == VK_LCONTROL.0 as u32 || vk == VK_RCONTROL.0 as u32 || vk == VK_CONTROL.0 as u32,
        _ => false,
    };
    let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

    if is_target {
        if down {
            // 自動リピートは無視（既に down 状態なら何もしない）。
            if !MOD_DOWN.swap(true, Relaxed) {
                POLLUTED.store(false, Relaxed);
            }
        } else if up {
            MOD_DOWN.store(false, Relaxed);
            if POLLUTED.load(Relaxed) {
                return; // 間に他キーが入った → タップ扱いしない
            }
            let now = Instant::now();
            let mut last = LAST_TAP.lock().unwrap();
            let is_double = last
                .map(|t| now.duration_since(t) <= Duration::from_millis(DOUBLE_TAP_MS))
                .unwrap_or(false);
            if is_double {
                *last = None;
                drop(last);
                trigger();
            } else {
                *last = Some(now);
            }
        }
    } else if down {
        // 対象でないキーが押されたらタップ列を無効化。
        POLLUTED.store(true, Relaxed);
    }
}

/// フックスレッドからメインスレッドへ渡してウィンドウをトグルする。
#[cfg(windows)]
fn trigger() {
    if let Some(app) = HOOK_APP.get() {
        let app2 = app.clone();
        let _ = app.run_on_main_thread(move || crate::system::window::toggle_window(&app2));
    }
}
