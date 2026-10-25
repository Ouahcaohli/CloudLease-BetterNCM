#![allow(non_snake_case)]

use betterncm_macro::{betterncm_main, betterncm_native_call};
use betterncm_plugin_api::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, RECT, SYSTEMTIME, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{BeginPaint, EndPaint, FillRect, GetStockObject, PAINTSTRUCT, HBRUSH, WHITE_BRUSH};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::Controls::InitCommonControls;

const CLASS_NAME: &[u16] = &[67,108,111,117,100,76,101,97,115,101,80,97,110,101,108,0];
const WM_APP_TOGGLE: u32 = WM_APP + 17;
// STATIC control left alignment is zero; windows-sys 0.59 does not expose SS_LEFT.
const SS_LEFT: u32 = 0;
// Win32 system color index for COLOR_WINDOW.
const COLOR_WINDOW: i32 = 5;
const ID_ACCOUNT: i32 = 101;
const ID_START: i32 = 102;
const ID_END: i32 = 103;
const ID_NOTE: i32 = 104;
const ID_ADD: i32 = 105;
const ID_DELETE: i32 = 106;
const ID_LIST: i32 = 107;
const ID_MINIMIZE: i32 = 108;

static PANEL_HWND: AtomicIsize = AtomicIsize::new(0);
static HOST_HWND: AtomicIsize = AtomicIsize::new(0);
static LEASES: OnceLock<Mutex<Vec<Lease>>> = OnceLock::new();

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Lease {
    account: String,
    start: String,
    end: String,
    note: String,
}

fn leases() -> &'static Mutex<Vec<Lease>> {
    LEASES.get_or_init(|| Mutex::new(load_leases()))
}

fn data_path() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("CloudLease").join("leases.json")
}

fn load_leases() -> Vec<Lease> {
    fs::read_to_string(data_path()).ok()
        .and_then(|s| serde_json::from_str::<Vec<Lease>>(&s).ok())
        .unwrap_or_default()
}

fn save_leases(items: &[Lease]) {
    let path = data_path();
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
    if let Ok(json) = serde_json::to_string_pretty(items) { let _ = fs::write(path, json); }
}

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

unsafe fn create_child(parent: HWND, class: &str, text: &str, style: u32, id: i32,
                       x: i32, y: i32, w: i32, h: i32) -> HWND {
    let c = wide(class); let t = wide(text);
    CreateWindowExW(0, c.as_ptr(), t.as_ptr(), style | WS_CHILD | WS_VISIBLE,
        x, y, w, h, parent, id as isize as HMENU, GetModuleHandleW(std::ptr::null()), std::ptr::null())
}

unsafe extern "system" fn find_host(hwnd: HWND, _: LPARAM) -> BOOL {
    if IsWindowVisible(hwnd) == 0 { return 1; }
    let mut title = [0u16; 512];
    let len = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
    if len <= 0 { return 1; }
    let text = String::from_utf16_lossy(&title[..len as usize]).to_lowercase();
    if text.contains("网易云音乐") || text.contains("cloudmusic") || text.contains("netease cloud music") {
        HOST_HWND.store(hwnd as isize, Ordering::SeqCst);
        return 0;
    }
    1
}

fn locate_host() -> HWND {
    HOST_HWND.store(0, Ordering::SeqCst);
    unsafe { EnumWindows(Some(find_host), 0); }
    HOST_HWND.load(Ordering::SeqCst) as HWND
}

unsafe fn position_panel(hwnd: HWND) {
    let host = locate_host();
    if host.is_null() || IsWindowVisible(host) == 0 || IsIconic(host) != 0 {
        ShowWindow(hwnd, SW_HIDE);
        return;
    }
    let mut r: RECT = std::mem::zeroed();
    if GetWindowRect(host, &mut r) == 0 { return; }
    let panel_w = 390;
    let mut x = r.left - panel_w;
    if x < 0 { x = r.right; }
    let height = (r.bottom - r.top).max(520);
    SetWindowPos(hwnd, HWND_TOPMOST, x, r.top, panel_w, height,
        SWP_NOACTIVATE);
}

unsafe fn refresh_list(hwnd: HWND) {
    let list = GetDlgItem(hwnd, ID_LIST);
    if list.is_null() { return; }
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    let items = leases().lock().unwrap_or_else(|e| e.into_inner()).clone();
    for item in items {
        let line = format!("{}   {} → {}{}", item.account, item.start, item.end,
            if item.note.is_empty() { String::new() } else { format!("   · {}", item.note) });
        let text = wide(&line);
        SendMessageW(list, LB_ADDSTRING, 0, text.as_ptr() as LPARAM);
    }
}

unsafe fn read_edit(hwnd: HWND, id: i32) -> String {
    let control = GetDlgItem(hwnd, id);
    let len = GetWindowTextLengthW(control).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    GetWindowTextW(control, buf.as_mut_ptr(), (buf.len()) as i32);
    String::from_utf16_lossy(&buf[..len]).trim().to_string()
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            InitCommonControls();
            create_child(hwnd, "STATIC", "日历", WS_VISIBLE | SS_LEFT, 120, 16, 10, 350, 22);
            create_child(hwnd, "SysMonthCal32", "", WS_VISIBLE, 128, 16, 34, 350, 190);
            create_child(hwnd, "STATIC", "", WS_VISIBLE | SS_LEFT, 121, 16, 228, 350, 24);
            create_child(hwnd, "STATIC", "租期记录", WS_VISIBLE | SS_LEFT, 122, 16, 258, 350, 24);
            create_child(hwnd, "STATIC", "账号 / 标识", SS_LEFT, 123, 16, 286, 92, 22);
            create_child(hwnd, "EDIT", "", WS_BORDER | ES_AUTOHSCROLL as u32, ID_ACCOUNT, 110, 282, 250, 24);
            create_child(hwnd, "STATIC", "开始日期", SS_LEFT, 124, 16, 316, 92, 22);
            create_child(hwnd, "EDIT", "YYYY-MM-DD", WS_BORDER | ES_AUTOHSCROLL as u32, ID_START, 110, 312, 112, 24);
            create_child(hwnd, "STATIC", "到期日期", SS_LEFT, 125, 16, 346, 92, 22);
            create_child(hwnd, "EDIT", "YYYY-MM-DD", WS_BORDER | ES_AUTOHSCROLL as u32, ID_END, 110, 342, 112, 24);
            create_child(hwnd, "STATIC", "备注", SS_LEFT, 126, 16, 376, 92, 22);
            create_child(hwnd, "EDIT", "", WS_BORDER | ES_AUTOHSCROLL as u32, ID_NOTE, 110, 372, 250, 24);
            create_child(hwnd, "BUTTON", "添加记录", BS_PUSHBUTTON as u32, ID_ADD, 16, 406, 100, 28);
            create_child(hwnd, "BUTTON", "删除选中", BS_PUSHBUTTON as u32, ID_DELETE, 124, 406, 100, 28);
            create_child(hwnd, "BUTTON", "收纳", BS_PUSHBUTTON as u32, ID_MINIMIZE, 232, 406, 80, 28);
            create_child(hwnd, "LISTBOX", "", WS_BORDER | LBS_NOTIFY as u32 | WS_VSCROLL | LBS_NOINTEGRALHEIGHT as u32, ID_LIST, 16, 446, 350, 340);
            SetTimer(hwnd, 1, 1000, None);
            refresh_list(hwnd);
            0
        }
        WM_TIMER => {
            let mut st: SYSTEMTIME = std::mem::zeroed();
            GetLocalTime(&mut st);
            let now = format!("{:04}-{:02}-{:02}   {:02}:{:02}:{:02}", st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond);
            let date = wide(&now);
            SetWindowTextW(GetDlgItem(hwnd, 121), date.as_ptr());
            position_panel(hwnd);
            0
        }
        WM_SIZE => {
            let h = (lparam >> 16) as i32;
            let w = (lparam & 0xffff) as i32;
            let list = GetDlgItem(hwnd, ID_LIST);
            if !list.is_null() { MoveWindow(list, 16, 446, (w - 32).max(180), (h - 462).max(100), 1); }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xffff) as i32;
            if id == ID_ADD {
                let account = read_edit(hwnd, ID_ACCOUNT);
                let start = read_edit(hwnd, ID_START);
                let end = read_edit(hwnd, ID_END);
                let note = read_edit(hwnd, ID_NOTE);
                if !account.is_empty() && !start.is_empty() && !end.is_empty() {
                    let mut data = leases().lock().unwrap_or_else(|e| e.into_inner());
                    data.push(Lease { account, start, end, note });
                    save_leases(&data);
                    drop(data);
                    refresh_list(hwnd);
                    SetWindowTextW(GetDlgItem(hwnd, ID_ACCOUNT), wide("").as_ptr());
                    SetWindowTextW(GetDlgItem(hwnd, ID_NOTE), wide("").as_ptr());
                } else {
                    MessageBoxW(hwnd, wide("请至少填写账号、开始日期和到期日期。日期格式建议 YYYY-MM-DD。 ").as_ptr(), wide("CloudLease").as_ptr(), MB_OK | MB_ICONINFORMATION);
                }
                return 0;
            }
            if id == ID_DELETE {
                let selected = SendMessageW(GetDlgItem(hwnd, ID_LIST), LB_GETCURSEL, 0, 0);
                if selected != LB_ERR as isize {
                    let mut data = leases().lock().unwrap_or_else(|e| e.into_inner());
                    if (selected as usize) < data.len() { data.remove(selected as usize); }
                    save_leases(&data);
                    drop(data);
                    refresh_list(hwnd);
                }
                return 0;
            }
            if id == ID_MINIMIZE { ShowWindow(hwnd, SW_HIDE); return 0; }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_APP_TOGGLE => {
            if IsWindowVisible(hwnd) != 0 { ShowWindow(hwnd, SW_HIDE); }
            else { position_panel(hwnd); ShowWindow(hwnd, SW_SHOW); SetForegroundWindow(hwnd); }
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let _ = GetStockObject(WHITE_BRUSH as i32);
            FillRect(hdc, &ps.rcPaint, GetStockObject(WHITE_BRUSH as i32) as HBRUSH);
            EndPaint(hwnd, &ps);
            0
        }
        WM_DESTROY => { KillTimer(hwnd, 1); PANEL_HWND.store(0, Ordering::SeqCst); 0 }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn ensure_window_thread() {
    if PANEL_HWND.load(Ordering::SeqCst) != 0 { return; }
    thread::spawn(|| unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as _,
            lpszMenuName: std::ptr::null(),
            lpszClassName: CLASS_NAME.as_ptr(),
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_TOPMOST, CLASS_NAME.as_ptr(),
            wide("CloudLease · 租期工作台").as_ptr(), WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME,
            0, 0, 390, 720, std::ptr::null_mut(), std::ptr::null_mut(), instance, std::ptr::null());
        if hwnd.is_null() { return; }
        PANEL_HWND.store(hwnd as isize, Ordering::SeqCst);
        ShowWindow(hwnd, SW_HIDE);
        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

#[betterncm_native_call]
fn toggle_panel() {
    ensure_window_thread();
    for _ in 0..30 {
        let hwnd = PANEL_HWND.load(Ordering::SeqCst) as HWND;
        if !hwnd.is_null() {
            unsafe { PostMessageW(hwnd, WM_APP_TOGGLE, 0, 0); }
            return;
        }
        thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[betterncm_main]
fn betterncm_plugin_main(mut ctx: PluginContext) {
    ctx.set_namespace("cloudlease_native");
    ctx.add_native_api(toggle_panel);
    println!("CloudLease native module loaded.");
}
