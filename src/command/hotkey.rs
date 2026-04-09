// 全局热键守护进程模块
//
// 通过 macOS CGEventTap 监听全局 Cmd+J 按键，
// 按下后聚焦已有终端窗口中的 j 交互模式，或新建一个。
//
// 用法:
//   j hotkey start   - 启动守护进程
//   j hotkey stop    - 停止守护进程
//   j hotkey status  - 查看状态

use crate::config::YamlConfig;
use crate::constants::HOTKEY_PID_FILE;
use crate::{error, info, usage};
use std::ffi::c_void;
use std::path::PathBuf;

// ========== macOS CGEventTap FFI ==========

// CGEventRef 本质是一个 CFTypeRef (opaque pointer)
type CGEventRef = *mut c_void;
type CFMachPortRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFAllocatorRef = *const c_void;
type CFStringRef = *const c_void;
type CGEventMask = u64;

/// macOS 虚拟键码: J = 0x26
const KEYCODE_J: i64 = 0x26;

/// CGEventType::KeyDown 的原始值
const K_CG_EVENT_KEY_DOWN: u32 = 10;

/// CGEventFlags::kCGEventFlagMaskCommand
const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;
/// CGEventFlags::kCGEventFlagMaskShift
const K_CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x00020000;
/// CGEventFlags::kCGEventFlagMaskAlternate
const K_CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x00080000;
/// CGEventFlags::kCGEventFlagMaskControl
const K_CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x00040000;

/// EventField::kCGKeyboardEventKeycode
const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

fn cg_event_mask_bit(event_type: u32) -> CGEventMask {
    1u64 << event_type
}

unsafe extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: CGEventMask,
        callback: extern "C" fn(
            proxy: *mut c_void,
            event_type: u32,
            event: CGEventRef,
            user_info: *mut c_void,
        ) -> CGEventRef,
        user_info: *mut c_void,
    ) -> CFMachPortRef;

    fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: i64,
    ) -> CFRunLoopSourceRef;

    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);

    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);

    fn CFRunLoopRun();
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;

    // CGEvent 字段读取
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventGetFlags(event: CGEventRef) -> u64;

    // kCFRunLoopCommonModes 全局字符串引用
    static kCFRunLoopCommonModes: CFStringRef;
}

// ========== 入口 ==========

pub fn handle_hotkey(action: &str, daemon: bool) {
    if daemon {
        run_daemon();
        return;
    }
    match action {
        "start" => handle_start(),
        "stop" => handle_stop(),
        "status" => handle_status(),
        _ => {
            usage!("j hotkey <start|stop|status>");
        }
    }
}

// ========== PID 文件管理 ==========

fn pid_file_path() -> PathBuf {
    YamlConfig::data_dir().join(HOTKEY_PID_FILE)
}

fn read_pid() -> Option<u32> {
    std::fs::read_to_string(pid_file_path())
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn write_pid(pid: u32) {
    let _ = std::fs::write(pid_file_path(), pid.to_string());
}

fn remove_pid() {
    let _ = std::fs::remove_file(pid_file_path());
}

fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

// ========== start / stop / status ==========

fn handle_start() {
    // 检查是否已在运行
    if let Some(pid) = read_pid() {
        if is_process_alive(pid) {
            info!("热键守护进程已在运行 (PID: {})", pid);
            return;
        }
        // 过期 PID 文件，清理
        remove_pid();
    }

    let exe = std::env::current_exe().expect("无法获取当前可执行文件路径");

    let child = std::process::Command::new(&exe)
        .args(["hotkey", "start", "--daemon"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    match child {
        Ok(child) => {
            let pid = child.id();
            write_pid(pid);
            info!("热键守护进程已启动 (PID: {})", pid);
            info!("  按 Cmd+J 可快速切换到 j 交互模式");
            info!("  注意: 需要在「系统设置 > 隐私与安全 > 辅助功能」中授权终端应用");
        }
        Err(e) => {
            error!("启动守护进程失败: {}", e);
        }
    }
}

fn handle_stop() {
    match read_pid() {
        Some(pid) => {
            if is_process_alive(pid) {
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
                std::thread::sleep(std::time::Duration::from_millis(300));
                if is_process_alive(pid) {
                    unsafe {
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }
                info!("热键守护进程已停止 (PID: {})", pid);
            } else {
                info!("热键守护进程未在运行（PID 文件已过期）");
            }
            remove_pid();
        }
        None => {
            info!("热键守护进程未在运行");
        }
    }
}

fn handle_status() {
    match read_pid() {
        Some(pid) if is_process_alive(pid) => {
            info!("热键守护进程正在运行 (PID: {})", pid);
        }
        Some(_) => {
            remove_pid();
            info!("热键守护进程未在运行");
        }
        None => {
            info!("热键守护进程未在运行");
        }
    }
}

// ========== 守护进程主循环 ==========

fn run_daemon() {
    // 安装 SIGTERM 处理器，退出时清理 PID 文件
    unsafe {
        libc::signal(
            libc::SIGTERM,
            sigterm_handler as *const () as libc::sighandler_t,
        );
    }

    let event_mask = cg_event_mask_bit(K_CG_EVENT_KEY_DOWN);

    let tap = unsafe {
        CGEventTapCreate(
            0, // kCGHIDEventTap
            0, // kCGHeadInsertEventTap
            0, // kCGEventTapOptionDefault (主动过滤，可吞事件)
            event_mask,
            event_tap_callback,
            std::ptr::null_mut(),
        )
    };

    if tap.is_null() {
        eprintln!("[hotkey] 无法创建全局事件监听器");
        eprintln!("[hotkey] 请在「系统设置 > 隐私与安全 > 辅助功能」中授权终端应用");
        remove_pid();
        std::process::exit(1);
    }

    unsafe {
        let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
        let run_loop = CFRunLoopGetCurrent();
        CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        CFRunLoopRun(); // 阻塞，直到进程被终止
    }
}

extern "C" fn sigterm_handler(_sig: libc::c_int) {
    remove_pid();
    std::process::exit(0);
}

// ========== 事件回调 ==========

extern "C" fn event_tap_callback(
    _proxy: *mut c_void,
    _event_type: u32,
    event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    let keycode = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) };
    let flags = unsafe { CGEventGetFlags(event) };

    let is_cmd = (flags & K_CG_EVENT_FLAG_MASK_COMMAND) != 0;
    let is_j = keycode == KEYCODE_J;
    let no_other_mods = (flags & K_CG_EVENT_FLAG_MASK_SHIFT) == 0
        && (flags & K_CG_EVENT_FLAG_MASK_ALTERNATE) == 0
        && (flags & K_CG_EVENT_FLAG_MASK_CONTROL) == 0;

    if is_cmd && is_j && no_other_mods {
        // 在新线程执行窗口操作，不阻塞 CFRunLoop
        std::thread::spawn(on_hotkey_pressed);
        // 返回 null 吞掉 Cmd+J 事件，不传递给前台应用
        return std::ptr::null_mut();
    }

    // 其他事件原样传递
    event
}

// ========== 热键触发逻辑 ==========

fn on_hotkey_pressed() {
    if !try_focus_existing_terminal() {
        open_new_terminal();
    }
}

/// 尝试聚焦已有终端中运行 j 的窗口
fn try_focus_existing_terminal() -> bool {
    // 尝试 iTerm2
    let iterm_script = r#"
tell application "System Events"
    if exists (process "iTerm2") then
        tell application "iTerm2"
            repeat with w in windows
                repeat with t in tabs of w
                    repeat with s in sessions of t
                        set sName to name of s
                        if sName contains "j >" or sName ends with "/j" then
                            select t
                            tell w to select
                            activate
                            return "found"
                        end if
                    end repeat
                end repeat
            end repeat
        end tell
    end if
end tell
return "none"
"#;

    if run_applescript(iterm_script) == "found" {
        return true;
    }

    // 尝试 Kitty
    if is_process_running("kitty") {
        // Kitty 暂时用简单方式：检查是否有 j 进程再激活
        if is_j_process_running() {
            let _ = run_applescript(r#"tell application "kitty" to activate"#);
            return true;
        }
    }

    // 尝试 Terminal.app
    let terminal_script = r#"
tell application "System Events"
    if exists (process "Terminal") then
        tell application "Terminal"
            repeat with w in windows
                repeat with t in tabs of w
                    if processes of t contains "j" then
                        set selected tab of w to t
                        set frontmost of w to true
                        activate
                        return "found"
                    end if
                end repeat
            end repeat
        end tell
    end if
end tell
return "none"
"#;

    if run_applescript(terminal_script) == "found" {
        return true;
    }

    false
}

/// 打开新终端窗口运行 j
fn open_new_terminal() {
    let j_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/usr/local/bin/j".to_string());

    // 优先 iTerm2 > Kitty > Terminal.app
    if std::path::Path::new("/Applications/iTerm.app").exists() {
        let script = format!(
            r#"
tell application "iTerm2"
    activate
    set newWindow to (create window with default profile)
    tell current session of newWindow
        write text "{}"
    end tell
end tell
"#,
            j_path
        );
        let _ = run_applescript(&script);
    } else if which_exists("kitty") {
        let _ = std::process::Command::new("kitty")
            .args(["--single-instance", "-e", &j_path])
            .spawn();
    } else {
        let script = format!(
            r#"
tell application "Terminal"
    activate
    do script "{}"
end tell
"#,
            j_path
        );
        let _ = run_applescript(&script);
    }
}

// ========== 工具函数 ==========

fn run_applescript(script: &str) -> String {
    std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn is_process_running(name: &str) -> bool {
    std::process::Command::new("pgrep")
        .args(["-x", name])
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_j_process_running() -> bool {
    // 检查是否有名为 j 的进程（交互模式）
    std::process::Command::new("pgrep")
        .args(["-x", "j"])
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn which_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
