#[cfg(target_os = "windows")]
use std::sync::Mutex;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    COLOR_BTNFACE, CreateFontW, DEFAULT_PITCH, DeleteObject, FF_DONTCARE, FW_NORMAL, HBRUSH,
    NONANTIALIASED_QUALITY,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::{
    ICC_STANDARD_CLASSES, ICC_WIN95_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CB_ADDSTRING, CB_GETCURSEL, CB_GETLBTEXT, CB_GETLBTEXTLEN, CB_RESETCONTENT,
    CB_SETCURSEL, CBN_DROPDOWN, CBS_DROPDOWNLIST, CS_HREDRAW, CS_VREDRAW, CreateMenu,
    CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW,
    GetCursorPos, GetDlgItem, GetMessageW, GetSystemMetrics, GetWindowTextLengthW, GetWindowTextW,
    HMENU, IDC_ARROW, IDI_APPLICATION, IDYES, LoadCursorW, LoadIconW, MB_ICONINFORMATION,
    MB_ICONQUESTION, MB_ICONWARNING, MB_OK, MB_YESNO, MF_CHECKED, MF_POPUP, MF_SEPARATOR,
    MF_STRING, MF_UNCHECKED, MessageBoxW, PostQuitMessage, RegisterClassW, SM_CXSCREEN,
    SM_CYSCREEN, SW_HIDE, SW_RESTORE, SW_SHOW, SendMessageW, SetForegroundWindow, SetTimer,
    SetWindowTextW, ShowWindow, TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WM_CLOSE,
    WM_COMMAND, WM_CREATE, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP, WM_SETFONT,
    WM_TIMER, WM_USER, WNDCLASSW, WS_BORDER, WS_CHILD, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

#[cfg(target_os = "windows")]
use crate::config::AppConfig;
#[cfg(target_os = "windows")]
use crate::cursor;
#[cfg(target_os = "windows")]
use crate::fix::{self, FixStatus};
#[cfg(target_os = "windows")]
use crate::process;

// Button control messages and check states
#[cfg(target_os = "windows")]
const BM_GETCHECK: u32 = 0x00F0;
#[cfg(target_os = "windows")]
const BM_SETCHECK: u32 = 0x00F1;
#[cfg(target_os = "windows")]
const BST_UNCHECKED: usize = 0;
#[cfg(target_os = "windows")]
const BST_CHECKED: usize = 1;

// ComboBox message introduced in ComCtl32 v6 to set minimum visible item count in dropdown
#[cfg(target_os = "windows")]
const CB_SETMINVISIBLE: u32 = 0x1701;

// ComComboBox notification codes
#[cfg(target_os = "windows")]
const CBN_SELCHANGE: u32 = 1;

// Custom Windows message ID for tray icon callbacks (WM_USER is base for application-defined messages)
#[cfg(target_os = "windows")]
const WM_TRAYICON: u32 = WM_USER + 101;
// Timer ID for periodic 1-second process monitoring
#[cfg(target_os = "windows")]
const TIMER_ID: usize = 1001;

// Control IDs (used in WM_COMMAND to identify which control triggered an event)
#[cfg(target_os = "windows")]
const ID_EDIT_PROCESS: usize = 201;
#[cfg(target_os = "windows")]
const ID_COMBO_SCHEME: usize = 202;
#[cfg(target_os = "windows")]
const ID_COMBO_SIZE: usize = 203;
#[cfg(target_os = "windows")]
const ID_CHK_AUTOSWITCH: usize = 204;
#[cfg(target_os = "windows")]
const ID_STATIC_STATUS: usize = 205;
#[cfg(target_os = "windows")]
const ID_BTN_APPLY: usize = 206;
#[cfg(target_os = "windows")]
const ID_BTN_RESTORE: usize = 207;
#[cfg(target_os = "windows")]
const ID_BTN_MINIMIZE: usize = 208;
#[cfg(target_os = "windows")]
const ID_BTN_EXIT: usize = 209;
#[cfg(target_os = "windows")]
const ID_BTN_FIX_STUTTER: usize = 210;
#[cfg(target_os = "windows")]
const ID_BTN_BROWSE_EXE: usize = 211;

// Tray context menu item IDs
#[cfg(target_os = "windows")]
const ID_TRAY_OPEN: usize = 301;
#[cfg(target_os = "windows")]
const ID_TRAY_AUTOSWITCH: usize = 302;
#[cfg(target_os = "windows")]
const ID_TRAY_APPLY: usize = 303;
#[cfg(target_os = "windows")]
const ID_TRAY_RESTORE: usize = 304;
#[cfg(target_os = "windows")]
const ID_TRAY_EXIT: usize = 305;
// Base IDs for dynamically generated tray submenus (schemes and sizes)
#[cfg(target_os = "windows")]
const ID_TRAY_SCHEME_BASE: usize = 4000;
#[cfg(target_os = "windows")]
const ID_TRAY_SIZE_BASE: usize = 5000;

// Thread-safe atomic booleans for tracking runtime state without locks:
// - IS_TARGET_RUNNING: Tracks whether monitored process was active during previous tick
// - HAS_SAVED_BACKUP: Ensures baseline cursor backup is created once on launch
#[cfg(target_os = "windows")]
static IS_TARGET_RUNNING: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static HAS_SAVED_BACKUP: AtomicBool = AtomicBool::new(false);

// Helper: Encodes UTF-8 string into null-terminated UTF-16 wide string for Win32 API calls
#[cfg(target_os = "windows")]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

// Helper: Reads text from a window or edit control handle (HWND) into a Rust String
#[cfg(target_os = "windows")]
fn get_window_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return String::new();
        }
        let mut buf: Vec<u16> = vec![0; (len + 1) as usize];
        GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

// Helper: Reads current selected text item from a ComboBox control handle
#[cfg(target_os = "windows")]
fn get_combobox_selected_text(hwnd: HWND) -> String {
    unsafe {
        let idx = SendMessageW(hwnd, CB_GETCURSEL, 0, 0) as usize;
        let len = SendMessageW(hwnd, CB_GETLBTEXTLEN, idx, 0);
        if len <= 0 {
            return String::new();
        }
        let mut buf: Vec<u16> = vec![0; (len + 1) as usize];
        SendMessageW(hwnd, CB_GETLBTEXT, idx, buf.as_mut_ptr() as LPARAM);
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

// Global application state protected by a Mutex for safe shared access across messages
#[cfg(target_os = "windows")]
struct AppState {
    schemes: Vec<String>,
    font: usize,
}

#[cfg(target_os = "windows")]
static GLOBAL_APP_STATE: Mutex<Option<AppState>> = Mutex::new(None);

// Main Window Procedure (WndProc) invoked by Windows for UI events
#[cfg(target_os = "windows")]
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // WM_CREATE: Sent once when the window is being created. Initialize controls and state here.
        WM_CREATE => {
            // Create clean Segoe UI font matching modern Windows dialogs
            let font = unsafe {
                CreateFontW(
                    -13,
                    0,
                    0,
                    0,
                    FW_NORMAL as i32,
                    0,
                    0,
                    0,
                    1,
                    0,
                    0,
                    NONANTIALIASED_QUALITY as u32,
                    DEFAULT_PITCH as u32 | FF_DONTCARE as u32,
                    to_wide("Segoe UI").as_ptr(),
                )
            };

            // Query initial available schemes from registry and cache in global AppState
            let schemes = cursor::get_available_schemes();
            if let Ok(mut lock) = GLOBAL_APP_STATE.lock() {
                *lock = Some(AppState {
                    schemes: schemes.clone(),
                    font: font as usize,
                });
            }

            let font_h = font;

            // Load persisted settings (or fallback defaults if settings.ini not found)
            let cfg = AppConfig::load();

            // Closure helper to instantiate child controls (buttons, edits, comboboxes, statics)
            let create_ctrl = |class: &str, text: &str, style: u32, x, y, w, h, id| unsafe {
                let hctrl = CreateWindowExW(
                    0,
                    to_wide(class).as_ptr(),
                    to_wide(text).as_ptr(),
                    WS_CHILD | WS_VISIBLE | style,
                    x,
                    y,
                    w,
                    h,
                    hwnd,
                    id as HMENU,
                    GetModuleHandleW(std::ptr::null()),
                    std::ptr::null_mut(),
                );
                SendMessageW(hctrl, WM_SETFONT, font_h as usize, 1);
                hctrl
            };

            // Labels and Inputs
            create_ctrl("STATIC", "Game Executable:", 0, 20, 20, 115, 20, 0);
            create_ctrl(
                "EDIT",
                &cfg.target_process,
                WS_BORDER | WS_TABSTOP | 0x0080, // ES_AUTOHSCROLL
                140,
                18,
                185,
                24,
                ID_EDIT_PROCESS,
            );
            create_ctrl(
                "BUTTON",
                "Browse...",
                WS_TABSTOP,
                330,
                18,
                70,
                24,
                ID_BTN_BROWSE_EXE,
            );

            // Cursor Scheme dropdown combobox
            // Note: WS_VSCROLL + CBS_DROPDOWNLIST gives the internal listbox vertical scrolling.
            // A drop height of 300 ensures plenty of vertical room for items.
            let hcombo_scheme = create_ctrl(
                "COMBOBOX",
                "",
                WS_TABSTOP | WS_VSCROLL | (CBS_DROPDOWNLIST as u32),
                140,
                54,
                260,
                300,
                ID_COMBO_SCHEME,
            );

            let active_scheme = cursor::get_current_active_scheme().unwrap_or_default();
            let mut selected_scheme_idx = 0;

            for (i, name) in schemes.iter().enumerate() {
                unsafe {
                    SendMessageW(
                        hcombo_scheme,
                        CB_ADDSTRING,
                        0,
                        to_wide(name).as_ptr() as LPARAM,
                    );
                }
                // Prefer user saved scheme; fallback to active system scheme
                if !cfg.scheme.is_empty() && name.eq_ignore_ascii_case(&cfg.scheme) {
                    selected_scheme_idx = i;
                } else if cfg.scheme.is_empty()
                    && !active_scheme.is_empty()
                    && name.eq_ignore_ascii_case(&active_scheme)
                {
                    selected_scheme_idx = i;
                }
            }
            if !schemes.is_empty() {
                unsafe {
                    SendMessageW(hcombo_scheme, CB_SETCURSEL, selected_scheme_idx, 0);
                    // CB_SETMINVISIBLE tells ComCtl32 v6 to expand visible items up to 15 before scrolling
                    SendMessageW(hcombo_scheme, CB_SETMINVISIBLE, 15, 0);
                }
            }

            create_ctrl("STATIC", "Cursor Size:", 0, 20, 92, 110, 20, 0);
            let hcombo_size = create_ctrl(
                "COMBOBOX",
                "",
                WS_TABSTOP | WS_VSCROLL | (CBS_DROPDOWNLIST as u32),
                140,
                90,
                260,
                200,
                ID_COMBO_SIZE,
            );

            let sizes = [
                "1 - Normal (32px)",
                "2 - Medium (48px)",
                "3 - Large (64px)",
                "4 - Extra Large (80px)",
                "5 - Huge (96px)",
                "6 - Maximum (112px)",
            ];
            // Use saved config size; fallback to active Windows accessibility size
            let current_size = if cfg.size >= 1 && cfg.size <= 6 {
                cfg.size
            } else {
                cursor::get_current_cursor_size().clamp(1, 6)
            };
            let selected_size_idx = (current_size.saturating_sub(1) as usize).min(sizes.len() - 1);

            for (i, s) in sizes.iter().enumerate() {
                unsafe {
                    SendMessageW(hcombo_size, CB_ADDSTRING, 0, to_wide(s).as_ptr() as LPARAM);
                    if i == selected_size_idx {
                        SendMessageW(hcombo_size, CB_SETCURSEL, i, 0);
                    }
                }
            }
            unsafe {
                SendMessageW(hcombo_size, CB_SETMINVISIBLE, 8, 0);
            }

            let hchk = create_ctrl(
                "BUTTON",
                "Auto-switch when target process is running",
                0x0003 | WS_TABSTOP, // BS_AUTOCHECKBOX
                20,
                130,
                380,
                24,
                ID_CHK_AUTOSWITCH,
            );
            unsafe {
                let check_state = if cfg.auto_switch {
                    BST_CHECKED
                } else {
                    BST_UNCHECKED
                };
                SendMessageW(hchk, BM_SETCHECK, check_state, 0);
            }

            create_ctrl(
                "STATIC",
                "Status: Monitoring (Waiting for target process)",
                0,
                20,
                166,
                380,
                22,
                ID_STATIC_STATUS,
            );

            // Action buttons
            create_ctrl(
                "BUTTON",
                "Apply Now",
                WS_TABSTOP,
                20,
                200,
                115,
                30,
                ID_BTN_APPLY,
            );
            create_ctrl(
                "BUTTON",
                "Restore",
                WS_TABSTOP,
                145,
                200,
                115,
                30,
                ID_BTN_RESTORE,
            );
            create_ctrl(
                "BUTTON",
                "Hide to Tray",
                WS_TABSTOP,
                270,
                200,
                130,
                30,
                ID_BTN_MINIMIZE,
            );

            // Arknights Stutter Fix control
            create_ctrl(
                "BUTTON",
                "Fix Mouse Stutter (Hardware Cursor)",
                WS_TABSTOP,
                20,
                240,
                380,
                30,
                ID_BTN_FIX_STUTTER,
            );
            update_fix_button_state(hwnd, Some(&cfg.game_path));

            // Save initial cursor backup on startup
            cursor::save_current();
            HAS_SAVED_BACKUP.store(true, Ordering::SeqCst);

            // Add tray icon
            add_tray_icon(hwnd);

            // Start 1-second monitoring timer
            unsafe {
                SetTimer(hwnd, TIMER_ID, 1000, None);
            }

            0
        }

        // WM_COMMAND: Triggered when user clicks buttons, selects comboboxes, or interacts with menus
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as usize;
            let code = ((wparam >> 16) & 0xFFFF) as u32;

            // CBN_DROPDOWN: Combobox dropdown is about to open.
            // Check if registry schemes changed since launch; only repopulate if changed to keep animation fast.
            if id == ID_COMBO_SCHEME && code == CBN_DROPDOWN {
                let new_schemes = cursor::get_available_schemes();
                let needs_update = if let Ok(lock) = GLOBAL_APP_STATE.lock() {
                    lock.as_ref()
                        .map(|st| st.schemes != new_schemes)
                        .unwrap_or(true)
                } else {
                    false
                };

                if needs_update {
                    let hcombo = unsafe { GetDlgItem(hwnd, ID_COMBO_SCHEME as i32) };
                    let current_selected = get_combobox_selected_text(hcombo);
                    if let Ok(mut lock) = GLOBAL_APP_STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.schemes = new_schemes.clone();
                        }
                    }
                    unsafe {
                        SendMessageW(hcombo, CB_RESETCONTENT, 0, 0);
                    }
                    let mut selected_idx = 0;
                    for (i, name) in new_schemes.iter().enumerate() {
                        unsafe {
                            SendMessageW(hcombo, CB_ADDSTRING, 0, to_wide(name).as_ptr() as LPARAM);
                        }
                        if !current_selected.is_empty()
                            && name.eq_ignore_ascii_case(&current_selected)
                        {
                            selected_idx = i;
                        }
                    }
                    if !new_schemes.is_empty() {
                        unsafe {
                            SendMessageW(hcombo, CB_SETCURSEL, selected_idx, 0);
                        }
                    }
                }
                return 0;
            }

            // Auto-save when user changes combobox selection (scheme or size)
            if (id == ID_COMBO_SCHEME || id == ID_COMBO_SIZE) && code == CBN_SELCHANGE {
                save_current_settings(hwnd);
                return 0;
            }

            match id {
                ID_CHK_AUTOSWITCH => {
                    save_current_settings(hwnd);
                }
                ID_BTN_APPLY | ID_TRAY_APPLY => {
                    apply_selected_settings(hwnd);
                    save_current_settings(hwnd);
                }
                ID_BTN_RESTORE | ID_TRAY_RESTORE => {
                    cursor::restore_scheme();
                    set_status(hwnd, "Status: Restored to defaults.");
                }
                ID_BTN_MINIMIZE => unsafe {
                    save_current_settings(hwnd);
                    ShowWindow(hwnd, SW_HIDE);
                },
                ID_BTN_FIX_STUTTER => {
                    let mut cfg = AppConfig::load();
                    if let Some(path) = fix::locate_asset_file(Some(&cfg.game_path)) {
                        let status = fix::get_status_of_file(&path);
                        match status {
                            FixStatus::ActiveSoftwareCursor => {
                                let title = "Disable PRTS Cursor";
                                let target_path = path.with_extension("bin.disabled");
                                let text = format!(
                                    "This will disable PRTS cursor.\n\nSource file:\n{}\n\nWill be renamed to:\n{}\n\nWhy:\nArknights renders a PRTS cursor that causes mouse lag/stutter during loading screen.\n\nDo you want to disable PRTS cursor?",
                                    path.display(),
                                    target_path.display()
                                );
                                let choice = unsafe {
                                    MessageBoxW(
                                        hwnd,
                                        to_wide(&text).as_ptr(),
                                        to_wide(title).as_ptr(),
                                        MB_YESNO | MB_ICONQUESTION,
                                    )
                                };
                                if choice == IDYES as i32 {
                                    match fix::toggle_fix(&path) {
                                        Ok((_, new_path)) => {
                                            if let Some(parent) = new_path.parent() {
                                                cfg.game_path =
                                                    parent.to_string_lossy().to_string();
                                                cfg.save();
                                            }
                                            update_fix_button_state(hwnd, Some(&cfg.game_path));
                                            set_status(hwnd, "Status: PRTS cursor disabled.");
                                            unsafe {
                                                MessageBoxW(
                                                    hwnd,
                                                    to_wide(&format!(
                                                        "PRTS cursor disabled!\n\nRenamed:\n{}\n\nWindows hardware cursor will be used for smooth mouse movement.\n(Please restart Arknights PC Client).",
                                                        new_path.display()
                                                    )).as_ptr(),
                                                    to_wide("PRTS Cursor Disabled").as_ptr(),
                                                    MB_OK | MB_ICONINFORMATION,
                                                );
                                            }
                                        }
                                        Err(e) => unsafe {
                                            MessageBoxW(
                                                hwnd,
                                                to_wide(&format!("Failed to apply fix:\n{e}"))
                                                    .as_ptr(),
                                                to_wide("Error").as_ptr(),
                                                MB_OK | MB_ICONWARNING,
                                            );
                                        },
                                    }
                                }
                            }
                            FixStatus::FixedDisabled => {
                                let title = "Restore PRTS Cursor";
                                let target_path = path
                                    .parent()
                                    .map(|p| p.join(fix::ASSET_FILENAME))
                                    .unwrap_or_else(|| path.clone());
                                let text = format!(
                                    "This will restore PRTS cursor.\n\nSource file:\n{}\n\nWill be renamed back to:\n{}\n\nNote:\nThis will re-enable PRTS cursor (mouse stuttering may return).\n\nDo you want to restore it?",
                                    path.display(),
                                    target_path.display()
                                );
                                let choice = unsafe {
                                    MessageBoxW(
                                        hwnd,
                                        to_wide(&text).as_ptr(),
                                        to_wide(title).as_ptr(),
                                        MB_YESNO | MB_ICONQUESTION,
                                    )
                                };
                                if choice == IDYES as i32 {
                                    match fix::toggle_fix(&path) {
                                        Ok((_, new_path)) => {
                                            if let Some(parent) = new_path.parent() {
                                                cfg.game_path =
                                                    parent.to_string_lossy().to_string();
                                                cfg.save();
                                            }
                                            update_fix_button_state(hwnd, Some(&cfg.game_path));
                                            set_status(
                                                hwnd,
                                                "Status: PRTS cursor restored (.bin active).",
                                            );
                                            unsafe {
                                                MessageBoxW(
                                                    hwnd,
                                                    to_wide(&format!(
                                                        "PRTS cursor restored!\n\nRenamed:\n{}\n\nThe software cursor file is active again.\n(Please restart Arknights if it was running).",
                                                        new_path.display()
                                                    )).as_ptr(),
                                                    to_wide("PRTS Cursor Restored").as_ptr(),
                                                    MB_OK | MB_ICONINFORMATION,
                                                );
                                            }
                                        }
                                        Err(e) => unsafe {
                                            MessageBoxW(
                                                hwnd,
                                                to_wide(&format!("Failed to restore cursor:\n{e}"))
                                                    .as_ptr(),
                                                to_wide("Error").as_ptr(),
                                                MB_OK | MB_ICONWARNING,
                                            );
                                        },
                                    }
                                }
                            }
                            FixStatus::NotFound => {
                                update_fix_button_state(hwnd, None);
                            }
                        }
                    }
                }
                ID_BTN_BROWSE_EXE => {
                    if let Some(selected) = open_file_dialog(hwnd) {
                        let mut cfg = AppConfig::load();
                        let p = std::path::PathBuf::from(&selected);
                        if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
                            if file_name.ends_with(".exe") {
                                cfg.target_process = file_name.to_string();
                                unsafe {
                                    SetWindowTextW(
                                        GetDlgItem(hwnd, ID_EDIT_PROCESS as i32),
                                        to_wide(file_name).as_ptr(),
                                    );
                                }
                            }
                        }
                        cfg.game_path = selected.clone();
                        cfg.save();
                        update_fix_button_state(hwnd, Some(&selected));

                        // if let Some(path) = fix::locate_asset_file(Some(&selected)) {
                        //     let status_str = match fix::get_status_of_file(&path) {
                        //         FixStatus::ActiveSoftwareCursor => {
                        //             "Game located. Stutter fix ready (.bin active)."
                        //         }
                        //         FixStatus::FixedDisabled => {
                        //             "Game located. Hardware cursor already active (.disabled)."
                        //         }
                        //         FixStatus::NotFound => "Game path saved.",
                        //     };
                        //     set_status(hwnd, &format!("Status: {status_str}"));
                        // } else {
                        //     set_status(
                        //         hwnd,
                        //         "Status: Executable path saved (asset bundle not found in folder).",
                        //     );
                        // }

                        set_status(hwnd, "Status: Executable path saved.");
                    }
                }
                ID_BTN_EXIT | ID_TRAY_EXIT => {
                    save_current_settings(hwnd);
                    remove_tray_icon(hwnd);
                    unsafe {
                        DestroyWindow(hwnd);
                    }
                }
                ID_TRAY_OPEN => unsafe {
                    ShowWindow(hwnd, SW_RESTORE);
                    SetForegroundWindow(hwnd);
                },
                ID_TRAY_AUTOSWITCH => {
                    let hchk = unsafe { GetDlgItem(hwnd, ID_CHK_AUTOSWITCH as i32) };
                    let checked = unsafe { SendMessageW(hchk, BM_GETCHECK, 0, 0) } as usize;
                    let new_state = if checked == BST_CHECKED {
                        BST_UNCHECKED
                    } else {
                        BST_CHECKED
                    };
                    unsafe {
                        SendMessageW(hchk, BM_SETCHECK, new_state, 0);
                    }
                    save_current_settings(hwnd);
                    if new_state == BST_CHECKED {
                        set_status(hwnd, "Status: Auto-switch enabled.");
                    } else {
                        set_status(hwnd, "Status: Auto-switch paused.");
                    }
                }
                // Tray dynamic scheme selection menu items
                id if id >= ID_TRAY_SCHEME_BASE && id < ID_TRAY_SIZE_BASE => {
                    let idx = id - ID_TRAY_SCHEME_BASE;
                    let scheme_opt = if let Ok(lock) = GLOBAL_APP_STATE.lock() {
                        lock.as_ref().and_then(|st| st.schemes.get(idx).cloned())
                    } else {
                        None
                    };

                    if let Some(scheme) = scheme_opt {
                        let hcombo = unsafe { GetDlgItem(hwnd, ID_COMBO_SCHEME as i32) };
                        unsafe {
                            SendMessageW(hcombo, CB_SETCURSEL, idx, 0);
                        }
                        let size_idx = unsafe {
                            SendMessageW(GetDlgItem(hwnd, ID_COMBO_SIZE as i32), CB_GETCURSEL, 0, 0)
                                as u32
                        };
                        let size = size_idx + 1;
                        cursor::apply_scheme_and_size(&scheme, size);
                        save_current_settings(hwnd);
                        set_status(
                            hwnd,
                            &format!("Status: Switched to {scheme} (size {size})."),
                        );
                    }
                }
                // Tray dynamic size selection menu items
                id if id >= ID_TRAY_SIZE_BASE => {
                    let size = (id - ID_TRAY_SIZE_BASE + 1) as u32;
                    let hcombo = unsafe { GetDlgItem(hwnd, ID_COMBO_SIZE as i32) };
                    unsafe {
                        SendMessageW(hcombo, CB_SETCURSEL, (size - 1) as usize, 0);
                    }
                    let scheme = get_combobox_selected_text(unsafe {
                        GetDlgItem(hwnd, ID_COMBO_SCHEME as i32)
                    });
                    if !scheme.is_empty() {
                        cursor::apply_scheme_and_size(&scheme, size);
                    } else {
                        cursor::set_cursor_size(size);
                    }
                    save_current_settings(hwnd);
                    set_status(hwnd, &format!("Status: Cursor size set to {size}."));
                }
                _ => {}
            }
            0
        }

        // WM_TIMER: Dispatched every 1000ms by SetTimer.
        // Checks if target process is running and automatically toggles cursor scheme accordingly.
        WM_TIMER => {
            let hchk = unsafe { GetDlgItem(hwnd, ID_CHK_AUTOSWITCH as i32) };
            let is_auto = unsafe { SendMessageW(hchk, BM_GETCHECK, 0, 0) } as usize == BST_CHECKED;
            if is_auto {
                let proc_name =
                    get_window_text(unsafe { GetDlgItem(hwnd, ID_EDIT_PROCESS as i32) });
                if !proc_name.trim().is_empty() {
                    let running = process::is_running(proc_name.trim());
                    let was_running = IS_TARGET_RUNNING.load(Ordering::SeqCst);

                    if running && !was_running {
                        if !HAS_SAVED_BACKUP.load(Ordering::SeqCst) {
                            cursor::save_current();
                            HAS_SAVED_BACKUP.store(true, Ordering::SeqCst);
                        }
                        apply_selected_settings(hwnd);
                        set_status(hwnd, &format!("Status: Active ({proc_name} running)"));
                        IS_TARGET_RUNNING.store(true, Ordering::SeqCst);
                    } else if !running && was_running {
                        cursor::restore_scheme();
                        set_status(
                            hwnd,
                            &format!("Status: Idle (Restored, watching {proc_name})"),
                        );
                        IS_TARGET_RUNNING.store(false, Ordering::SeqCst);
                    }
                }
            }
            0
        }

        // WM_TRAYICON: Custom callback received from system tray icon notifications
        WM_TRAYICON => {
            let event = (lparam & 0xFFFF) as u32;
            match event {
                // Left click or double click on tray icon -> restore and focus main window
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => unsafe {
                    ShowWindow(hwnd, SW_RESTORE);
                    SetForegroundWindow(hwnd);
                },
                // Right click on tray icon -> show context popup menu
                WM_RBUTTONUP => {
                    show_tray_menu(hwnd);
                }
                _ => {}
            }
            0
        }

        // WM_CLOSE: User clicked 'X' on window caption -> auto-save, remove tray icon, and destroy window to quit
        WM_CLOSE => {
            save_current_settings(hwnd);
            remove_tray_icon(hwnd);
            unsafe {
                // DestroyWindow sends WM_DESTROY and closes the entire process (including attached CMD/terminal).
                // If you want 'X' to minimize to tray instead of quitting, use `ShowWindow(hwnd, SW_HIDE);` here.
                DestroyWindow(hwnd);
            }
            0
        }

        // WM_DESTROY: Window is being destroyed -> auto-save, cleanup tray icon, fonts, and post quit message
        WM_DESTROY => {
            save_current_settings(hwnd);
            remove_tray_icon(hwnd);
            if let Ok(mut lock) = GLOBAL_APP_STATE.lock() {
                if let Some(state) = lock.take() {
                    unsafe {
                        DeleteObject(state.font as _);
                    }
                }
            }
            unsafe {
                PostQuitMessage(0);
            }
            0
        }

        // Default handler for unhandled messages
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

// Reads UI selection from GUI comboboxes and applies scheme + size
#[cfg(target_os = "windows")]
fn apply_selected_settings(hwnd: HWND) {
    let scheme = get_combobox_selected_text(unsafe { GetDlgItem(hwnd, ID_COMBO_SCHEME as i32) });
    let size_idx =
        unsafe { SendMessageW(GetDlgItem(hwnd, ID_COMBO_SIZE as i32), CB_GETCURSEL, 0, 0) as u32 };
    let size = size_idx + 1;

    if !scheme.is_empty() {
        cursor::apply_scheme_and_size(&scheme, size);
    } else {
        cursor::set_cursor_size(size);
    }
    set_status(hwnd, &format!("Status: Applied {scheme} (size {size})"));
}

// Gathers current UI control values and writes them to settings.ini next to the executable
#[cfg(target_os = "windows")]
fn save_current_settings(hwnd: HWND) {
    let target_process = get_window_text(unsafe { GetDlgItem(hwnd, ID_EDIT_PROCESS as i32) });
    let scheme = get_combobox_selected_text(unsafe { GetDlgItem(hwnd, ID_COMBO_SCHEME as i32) });
    let size_idx =
        unsafe { SendMessageW(GetDlgItem(hwnd, ID_COMBO_SIZE as i32), CB_GETCURSEL, 0, 0) as u32 };
    let size = (size_idx + 1).clamp(1, 6);
    let hchk = unsafe { GetDlgItem(hwnd, ID_CHK_AUTOSWITCH as i32) };
    let auto_switch = (unsafe { SendMessageW(hchk, BM_GETCHECK, 0, 0) } as usize) == BST_CHECKED;

    let existing = AppConfig::load();
    let cfg = AppConfig {
        target_process: if target_process.trim().is_empty() {
            String::from("Arknights.exe")
        } else {
            target_process.trim().to_string()
        },
        scheme,
        size,
        auto_switch,
        game_path: existing.game_path,
    };
    cfg.save();
}

// Updates the fix button enabled state and text depending on whether the game asset is found
#[cfg(target_os = "windows")]
fn update_fix_button_state(hwnd: HWND, custom_path: Option<&str>) {
    let hbtn = unsafe { GetDlgItem(hwnd, ID_BTN_FIX_STUTTER as i32) };
    if hbtn.is_null() {
        return;
    }
    if let Some(path) = fix::locate_asset_file(custom_path) {
        match fix::get_status_of_file(&path) {
            FixStatus::ActiveSoftwareCursor => unsafe {
                SetWindowTextW(hbtn, to_wide("Disable PRTS Cursor").as_ptr());
                EnableWindow(hbtn, 1);
            },
            FixStatus::FixedDisabled => unsafe {
                SetWindowTextW(hbtn, to_wide("Restore PRTS Cursor").as_ptr());
                EnableWindow(hbtn, 1);
            },
            FixStatus::NotFound => unsafe {
                SetWindowTextW(
                    hbtn,
                    to_wide("Restore PRTS Cursor (Locate .exe first)").as_ptr(),
                );
                EnableWindow(hbtn, 0);
            },
        }
    } else {
        unsafe {
            SetWindowTextW(
                hbtn,
                to_wide("Disable PRTS Cursor (Locate .exe first)").as_ptr(),
            );
            EnableWindow(hbtn, 0);
        }
    }
}

// Opens native Windows File Open Dialog to locate Arknights executable or .bin file
#[cfg(target_os = "windows")]
fn open_file_dialog(hwnd: HWND) -> Option<String> {
    let mut file_buf = vec![0u16; 1024];
    let filter = "Arknights Game / Data Files\0Arknights.exe;*.bin;*.bin.disabled;*.exe\0All Files (*.*)\0*.*\0\0";
    let filter_wide: Vec<u16> = filter.encode_utf16().chain(Some(0)).collect();
    let title = to_wide("Select Arknights Executable or Data Folder");

    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = hwnd;
    ofn.lpstrFilter = filter_wide.as_ptr();
    ofn.lpstrFile = file_buf.as_mut_ptr();
    ofn.nMaxFile = file_buf.len() as u32;
    ofn.lpstrTitle = title.as_ptr();
    ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST;

    let res = unsafe { GetOpenFileNameW(&mut ofn) };
    if res != 0 {
        let len = file_buf
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(file_buf.len());
        Some(String::from_utf16_lossy(&file_buf[..len]))
    } else {
        None
    }
}

// Updates static text status label in the GUI
#[cfg(target_os = "windows")]
fn set_status(hwnd: HWND, text: &str) {
    unsafe {
        let hstatus = GetDlgItem(hwnd, ID_STATIC_STATUS as i32);
        SetWindowTextW(hstatus, to_wide(text).as_ptr());
    }
}

// Registers taskbar notification area (system tray) icon with Windows Shell
#[cfg(target_os = "windows")]
fn add_tray_icon(hwnd: HWND) {
    unsafe {
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);

        let tip = to_wide("Cursor Switcher");
        let len = tip.len().min(nid.szTip.len());
        nid.szTip[..len].copy_from_slice(&tip[..len]);

        Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

// Unregisters and removes the system tray icon on shutdown
#[cfg(target_os = "windows")]
fn remove_tray_icon(hwnd: HWND) {
    unsafe {
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

// Displays context popup menu on right-click of tray icon
#[cfg(target_os = "windows")]
fn show_tray_menu(hwnd: HWND) {
    unsafe {
        let hmenu = CreatePopupMenu();
        let hschemes_menu = CreateMenu();
        let hsizes_menu = CreateMenu();

        AppendMenuW(
            hmenu,
            MF_STRING,
            ID_TRAY_OPEN,
            to_wide("Open Window").as_ptr(),
        );
        AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());

        let hchk = GetDlgItem(hwnd, ID_CHK_AUTOSWITCH as i32);
        let is_auto = SendMessageW(hchk, BM_GETCHECK, 0, 0) as usize == BST_CHECKED;
        let auto_flag = if is_auto { MF_CHECKED } else { MF_UNCHECKED };
        AppendMenuW(
            hmenu,
            MF_STRING | auto_flag,
            ID_TRAY_AUTOSWITCH,
            to_wide("Auto-Switch Enabled").as_ptr(),
        );

        // Dynamically build scheme selection submenu
        let new_schemes = cursor::get_available_schemes();
        if let Ok(mut lock) = GLOBAL_APP_STATE.lock() {
            if let Some(state) = lock.as_mut() {
                state.schemes = new_schemes.clone();
            }
        }
        for (i, name) in new_schemes.iter().enumerate() {
            AppendMenuW(
                hschemes_menu,
                MF_STRING,
                ID_TRAY_SCHEME_BASE + i,
                to_wide(name).as_ptr(),
            );
        }
        AppendMenuW(
            hmenu,
            MF_POPUP,
            hschemes_menu as usize,
            to_wide("Select Scheme").as_ptr(),
        );

        // Build size selection submenu (1..6)
        for s in 1..=6 {
            AppendMenuW(
                hsizes_menu,
                MF_STRING,
                ID_TRAY_SIZE_BASE + (s - 1),
                to_wide(&format!("Size {s} ({}px)", 32 + (s - 1) * 16)).as_ptr(),
            );
        }
        AppendMenuW(
            hmenu,
            MF_POPUP,
            hsizes_menu as usize,
            to_wide("Select Size").as_ptr(),
        );

        AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            hmenu,
            MF_STRING,
            ID_TRAY_APPLY,
            to_wide("Apply Now").as_ptr(),
        );
        AppendMenuW(
            hmenu,
            MF_STRING,
            ID_TRAY_RESTORE,
            to_wide("Restore Defaults").as_ptr(),
        );
        AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(hmenu, MF_STRING, ID_TRAY_EXIT, to_wide("Exit").as_ptr());

        // Get cursor position to display popup menu right at mouse pointer
        let mut pt: POINT = std::mem::zeroed();
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            hmenu,
            TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        DestroyMenu(hmenu);
    }
}

// Entry point for the Windows GUI subsystem:
// 1. Inits Common Controls v6 (modern theme engine).
// 2. Registers the WNDCLASS window class.
// 3. Creates the top-level overlapped window.
// 4. Runs standard Win32 message pump (GetMessageW / TranslateMessage / DispatchMessageW).
#[cfg(target_os = "windows")]
pub fn run_gui() {
    unsafe {
        // Initialize Common Controls v6 classes (buttons, comboboxes, themes)
        let icex = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES | ICC_WIN95_CLASSES,
        };
        InitCommonControlsEx(&icex);

        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("CursorSwitcherWindowClass");

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc), // Register function pointer
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: LoadIconW(std::ptr::null_mut(), IDI_APPLICATION),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_BTNFACE + 1) as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        RegisterClassW(&wc);

        // Windows calls window_proc in two ways:
        // 1. Inside run_gui message pump (GetMessageW -> DispatchMessageW).
        // 2. Synchronous OS calls during API execution:
        //    - CreateWindowExW(...) calls window_proc with WM_CREATE before CreateWindowExW returns.
        //    - SendMessageW(...) bypasses queue and calls window_proc immediately.
        //    - ShowWindow(...) / DestroyWindow(...) calls window_proc directly for resize/close events.

        let win_w = 440;
        let win_h = 330;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - win_w) / 2;
        let y = (screen_h - win_h) / 2;

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            to_wide("Cursor Switcher").as_ptr(),
            0x00CA0000, // WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX
            x,
            y,
            win_w,
            win_h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );

        ShowWindow(hwnd, SW_SHOW);

        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg); // Windows calls window_proc here for queued events
        }
    }
}
