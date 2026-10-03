#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::path::Path;

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_DWORD, REG_EXPAND_SZ,
    REG_SZ, RegCloseKey, RegDeleteValueW, RegEnumValueW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CopyImage, IMAGE_CURSOR, LR_LOADFROMFILE, LR_SHARED, LoadImageW, OCR_APPSTARTING, OCR_CROSS,
    OCR_HAND, OCR_HELP, OCR_IBEAM, OCR_NO, OCR_NORMAL, OCR_SIZEALL, OCR_SIZENESW, OCR_SIZENS,
    OCR_SIZENWSE, OCR_SIZEWE, OCR_UP, OCR_WAIT, SetSystemCursor, SystemParametersInfoW,
};

// SystemParametersInfo action code to reload system cursors from registry
#[cfg(target_os = "windows")]
const SPI_SETCURSORS: u32 = 0x0057;

// Windows 10/11 OCR cursor IDs for location pin and person select (not exposed in older Win32 headers)
#[cfg(target_os = "windows")]
const OCR_PIN: u32 = 32671;
#[cfg(target_os = "windows")]
const OCR_PERSON: u32 = 32672;

// Mapping between registry value names and active Windows OCR_* cursor IDs.
// There are 16 active OCR IDs that can be directly replaced via SetSystemCursor.
#[cfg(target_os = "windows")]
const OCR_MAPPINGS: [(&str, u32); 16] = [
    ("Arrow", OCR_NORMAL),
    ("Help", OCR_HELP),
    ("AppStarting", OCR_APPSTARTING),
    ("Wait", OCR_WAIT),
    ("crosshair", OCR_CROSS),
    ("IBeam", OCR_IBEAM),
    ("No", OCR_NO),
    ("SizeNS", OCR_SIZENS),
    ("SizeWE", OCR_SIZEWE),
    ("SizeNWSE", OCR_SIZENWSE),
    ("SizeNESW", OCR_SIZENESW),
    ("SizeAll", OCR_SIZEALL),
    ("UpArrow", OCR_UP),
    ("Hand", OCR_HAND),
    ("Pin", OCR_PIN),
    ("Person", OCR_PERSON),
];

// Windows cursor slots in registry schemes (comma-separated string).
// Windows expects exactly 17 positional slots in this exact order:
// [0] Arrow, [1] Help, [2] AppStarting, [3] Wait, [4] crosshair, [5] IBeam,
// [6] NWPen, [7] No, [8] SizeNS, [9] SizeWE, [10] SizeNWSE, [11] SizeNESW,
// [12] SizeAll, [13] UpArrow, [14] Hand, [15] Pin, [16] Person.
#[cfg(target_os = "windows")]
const CURSOR_NAMES: [&str; 17] = [
    "Arrow",
    "Help",
    "AppStarting",
    "Wait",
    "crosshair",
    "IBeam",
    "NWPen",
    "No",
    "SizeNS",
    "SizeWE",
    "SizeNWSE",
    "SizeNESW",
    "SizeAll",
    "UpArrow",
    "Hand",
    "Pin",
    "Person",
];

// Converts a Rust &str (UTF-8) into a null-terminated UTF-16 buffer (Vec<u16>)
// required by Windows Win32 *W (wide) APIs.
#[cfg(target_os = "windows")]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

// Opens a Windows registry key in read-only mode.
// Returns Some(HKEY) on success (0 / ERROR_SUCCESS), None on error.
#[cfg(target_os = "windows")]
fn reg_open_read(root: HKEY, subkey: &str) -> Option<HKEY> {
    let wide = to_wide(subkey);
    let mut hkey: HKEY = std::ptr::null_mut();
    let res = unsafe { RegOpenKeyExW(root, wide.as_ptr(), 0, KEY_READ, &mut hkey) };
    if res == 0 { Some(hkey) } else { None }
}

// Opens a Windows registry key with read and write permissions.
#[cfg(target_os = "windows")]
fn reg_open_write(root: HKEY, subkey: &str) -> Option<HKEY> {
    let wide = to_wide(subkey);
    let mut hkey: HKEY = std::ptr::null_mut();
    let res = unsafe { RegOpenKeyExW(root, wide.as_ptr(), 0, KEY_READ | KEY_WRITE, &mut hkey) };
    if res == 0 { Some(hkey) } else { None }
}

// Reads a string value (REG_SZ or REG_EXPAND_SZ) from an open registry key.
// Two-step Win32 query: first call queries byte length, second call copies the data.
#[cfg(target_os = "windows")]
fn reg_get_string(hkey: HKEY, value_name: &str) -> Option<String> {
    let wide = to_wide(value_name);
    // An empty value_name refers to the key's "(Default)" unnamed value (pass NULL pointer)
    let name_ptr = if value_name.is_empty() {
        std::ptr::null()
    } else {
        wide.as_ptr()
    };

    let mut val_type: u32 = 0;
    let mut byte_len: u32 = 0;

    // Step 1: Query required buffer size (byte_len)
    let res = unsafe {
        RegQueryValueExW(
            hkey,
            name_ptr,
            std::ptr::null(),
            &mut val_type,
            std::ptr::null_mut(),
            &mut byte_len,
        )
    };
    if res != 0 || byte_len == 0 {
        return None;
    }

    // Step 2: Allocate byte buffer and fetch actual string data
    let mut buf = vec![0u8; byte_len as usize];
    let res = unsafe {
        RegQueryValueExW(
            hkey,
            name_ptr,
            std::ptr::null(),
            &mut val_type,
            buf.as_mut_ptr(),
            &mut byte_len,
        )
    };
    if res != 0 {
        return None;
    }

    // Convert raw u8 buffer into u16 slice (UTF-16), strip trailing null, and convert to String
    let u16_slice: &[u16] =
        unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u16, byte_len as usize / 2) };
    let null_pos = u16_slice
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(u16_slice.len());
    Some(String::from_utf16_lossy(&u16_slice[..null_pos]))
}

// Reads a 32-bit unsigned integer (REG_DWORD) from an open registry key.
#[allow(dead_code)]
#[cfg(target_os = "windows")]
fn reg_get_dword(hkey: HKEY, value_name: &str) -> Option<u32> {
    let wide = to_wide(value_name);
    let mut val_type: u32 = 0;
    let mut data: u32 = 0;
    let mut byte_len: u32 = std::mem::size_of::<u32>() as u32;

    let res = unsafe {
        RegQueryValueExW(
            hkey,
            wide.as_ptr(),
            std::ptr::null(),
            &mut val_type,
            &mut data as *mut u32 as *mut u8,
            &mut byte_len,
        )
    };
    if res == 0 && val_type == REG_DWORD {
        Some(data)
    } else {
        None
    }
}

// Writes a string value (REG_SZ or REG_EXPAND_SZ) to an open registry key.
#[cfg(target_os = "windows")]
fn reg_set_string(hkey: HKEY, value_name: &str, val: &str, is_expand: bool) -> bool {
    let wide_name = to_wide(value_name);
    let name_ptr = if value_name.is_empty() {
        std::ptr::null()
    } else {
        wide_name.as_ptr()
    };
    let wide_val = to_wide(val);
    let val_type = if is_expand { REG_EXPAND_SZ } else { REG_SZ };
    let byte_len = (wide_val.len() * std::mem::size_of::<u16>()) as u32;

    let res = unsafe {
        RegSetValueExW(
            hkey,
            name_ptr,
            0,
            val_type,
            wide_val.as_ptr() as *const u8,
            byte_len,
        )
    };
    res == 0
}

// Writes a 32-bit DWORD value to an open registry key.
#[cfg(target_os = "windows")]
fn reg_set_dword(hkey: HKEY, value_name: &str, val: u32) -> bool {
    let wide_name = to_wide(value_name);
    let res = unsafe {
        RegSetValueExW(
            hkey,
            wide_name.as_ptr(),
            0,
            REG_DWORD,
            &val as *const u32 as *const u8,
            std::mem::size_of::<u32>() as u32,
        )
    };
    res == 0
}

// Deletes a specific registry value under an open key.
#[cfg(target_os = "windows")]
fn reg_delete_value(hkey: HKEY, value_name: &str) -> bool {
    let wide_name = to_wide(value_name);
    let res = unsafe { RegDeleteValueW(hkey, wide_name.as_ptr()) };
    res == 0
}

// Finds the cursor scheme CSV definition string and source origin:
// - Scheme Source = 2: User custom scheme (from HKCU\Control Panel\Cursors\Schemes)
// - Scheme Source = 1: System built-in scheme (from HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Control Panel\Cursors\Schemes)
// - Scheme Source = 0: Default / Modified (None)
#[cfg(target_os = "windows")]
fn get_scheme_data_and_source(scheme_name: &str) -> Option<(String, u32)> {
    // 1. Check HKCU (user-created custom schemes)
    if let Some(hkey) = reg_open_read(HKEY_CURRENT_USER, r"Control Panel\Cursors\Schemes") {
        if let Some(data) = reg_get_string(hkey, scheme_name) {
            unsafe { RegCloseKey(hkey) };
            return Some((data, 2));
        }
        unsafe { RegCloseKey(hkey) };
    }

    // 2. Check HKLM (system built-in schemes)
    if let Some(hkey) = reg_open_read(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Control Panel\Cursors\Schemes",
    ) {
        if let Some(data) = reg_get_string(hkey, scheme_name) {
            unsafe { RegCloseKey(hkey) };
            return Some((data, 1));
        }
        unsafe { RegCloseKey(hkey) };
    }

    None
}

// Applies a cursor scheme by name and scale size:
// 1. Calculates pixel dimensions (32..112px) and accessibility index (1..16).
// 2. Writes all 17 cursor slots to HKCU\Control Panel\Cursors (REG_EXPAND_SZ).
// 3. Sets Scheme Source and CursorBaseSize in registry.
// 4. Updates CursorSize in HKCU\Software\Microsoft\Accessibility.
// 5. Calls apply_cursor_size to instantly inject newly scaled cursor handles into Windows.
#[cfg(target_os = "windows")]
pub fn apply_scheme_and_size(scheme_name: &str, size: u32) -> bool {
    println!("Applying scheme '{scheme_name}' with size {size}...");

    // Convert input size (1..16 or 32..112) into (slider_index, base_pixel_size)
    let (cursor_size, cursor_base_size) = if size <= 16 {
        let s = size.max(1);
        (s, 32 + (s - 1) * 16)
    } else {
        let s = ((size.saturating_sub(32)) / 16 + 1).clamp(1, 16);
        (s, size)
    };

    let (scheme_str, scheme_source) = match get_scheme_data_and_source(scheme_name) {
        Some((s, src)) if !s.trim().is_empty() => (s, src),
        _ => {
            println!("Cursor scheme not found: {scheme_name}");
            return false;
        }
    };

    // Scheme data is a comma-separated list of 17 cursor file paths
    let cursors: Vec<&str> = scheme_str.split(',').collect();
    if cursors.is_empty() {
        println!("Cursor scheme is empty.");
        return false;
    }

    // Write to HKCU\Control Panel\Cursors:
    // - Individual cursor slots: REG_EXPAND_SZ (may contain %SYSTEMROOT%)
    // - (Default) value: REG_SZ scheme name
    // - Scheme Source: REG_DWORD (1 = HKLM, 2 = HKCU)
    // - CursorBaseSize: REG_DWORD pixel size (32, 48, 64...)
    if let Some(hkey) = reg_open_write(HKEY_CURRENT_USER, r"Control Panel\Cursors") {
        for (i, &name) in CURSOR_NAMES.iter().enumerate() {
            let path = cursors.get(i).copied().unwrap_or("").trim();
            reg_set_string(hkey, name, path, true);
        }
        reg_set_string(hkey, "", scheme_name, false);
        reg_set_dword(hkey, "Scheme Source", scheme_source);
        reg_set_dword(hkey, "CursorBaseSize", cursor_base_size);
        unsafe { RegCloseKey(hkey) };
    }

    if let Some(hkey) = reg_open_write(HKEY_CURRENT_USER, r"Software\Microsoft\Accessibility") {
        reg_set_dword(hkey, "CursorSize", cursor_size);
        unsafe { RegCloseKey(hkey) };
    }

    // Directly load and set cursors at target size in ONE pass
    apply_cursor_size(cursor_base_size);

    println!("Cursor changed to {scheme_name} at {cursor_size} ({cursor_base_size}px)!");
    true
}

// Restores default Windows cursors:
// 1. Clears individual slot overrides from HKCU\Control Panel\Cursors.
// 2. Sets (Default) back to "Windows Default" and Scheme Source to 0.
// 3. Broadcasts SPI_SETCURSORS to notify Windows subsystem.
// 4. Reloads system cursors at base 32px.
#[cfg(target_os = "windows")]
pub fn restore_scheme() -> bool {
    println!("Restoring previous cursor configuration...");

    if let Some(hkey) = reg_open_write(HKEY_CURRENT_USER, r"Control Panel\Cursors") {
        for name in CURSOR_NAMES {
            reg_delete_value(hkey, name);
        }
        reg_set_string(hkey, "", "Windows Default", false);
        reg_set_dword(hkey, "Scheme Source", 0);
        reg_set_dword(hkey, "CursorBaseSize", 32);
        unsafe { RegCloseKey(hkey) };
    }

    if let Some(hkey) = reg_open_write(HKEY_CURRENT_USER, r"Software\Microsoft\Accessibility") {
        reg_set_dword(hkey, "CursorSize", 1);
        unsafe { RegCloseKey(hkey) };
    }

    unsafe {
        SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), 0);
    }

    apply_cursor_size(32);

    println!("Previous cursor configuration restored!");
    true
}

#[cfg(target_os = "windows")]
pub fn save_current() -> bool {
    true
}

// Sets cursor size without switching current active scheme.
#[cfg(target_os = "windows")]
pub fn set_cursor_size(size: u32) -> bool {
    let (cursor_size, cursor_base_size) = if size <= 16 {
        let s = size.max(1);
        (s, 32 + (s - 1) * 16)
    } else {
        let s = ((size.saturating_sub(32)) / 16 + 1).clamp(1, 16);
        (s, size)
    };

    if let Some(hkey) = reg_open_write(HKEY_CURRENT_USER, r"Software\Microsoft\Accessibility") {
        reg_set_dword(hkey, "CursorSize", cursor_size);
        unsafe { RegCloseKey(hkey) };
    }

    if let Some(hkey) = reg_open_write(HKEY_CURRENT_USER, r"Control Panel\Cursors") {
        reg_set_dword(hkey, "CursorBaseSize", cursor_base_size);
        unsafe { RegCloseKey(hkey) };
    }

    apply_cursor_size(cursor_base_size);
    println!("Cursor size changed to {cursor_size} (base size: {cursor_base_size}px).");
    true
}

// Enumerates all available cursor schemes from both HKCU (user) and HKLM (system) registry.
// Uses RegEnumValueW in a loop until error (ERROR_NO_MORE_ITEMS / 259).
#[cfg(target_os = "windows")]
pub fn get_available_schemes() -> Vec<String> {
    let mut schemes = Vec::new();

    // Query HKCU\Control Panel\Cursors\Schemes (User-installed schemes)
    if let Some(hkey) = reg_open_read(HKEY_CURRENT_USER, r"Control Panel\Cursors\Schemes") {
        let mut index = 0;
        let mut name_buf = vec![0u16; 512];
        loop {
            let mut name_len = name_buf.len() as u32;
            let res = unsafe {
                RegEnumValueW(
                    hkey,
                    index,
                    name_buf.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if res != 0 {
                break;
            }
            let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            if !name.is_empty() && !schemes.contains(&name) {
                schemes.push(name);
            }
            index += 1;
        }
        unsafe { RegCloseKey(hkey) };
    }

    // Query HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Control Panel\Cursors\Schemes (System schemes)
    if let Some(hkey) = reg_open_read(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Control Panel\Cursors\Schemes",
    ) {
        let mut index = 0;
        let mut name_buf = vec![0u16; 512];
        loop {
            let mut name_len = name_buf.len() as u32;
            let res = unsafe {
                RegEnumValueW(
                    hkey,
                    index,
                    name_buf.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if res != 0 {
                break;
            }
            let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            if !name.is_empty() && !schemes.contains(&name) {
                schemes.push(name);
            }
            index += 1;
        }
        unsafe { RegCloseKey(hkey) };
    }

    // Sort schemes alphabetically (case-insensitive) for clean dropdown display
    schemes.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    schemes
}

// Queries the currently active cursor scheme name from HKCU\Control Panel\Cursors (Default value).
#[allow(dead_code)]
#[cfg(target_os = "windows")]
pub fn get_current_active_scheme() -> Option<String> {
    if let Some(hkey) = reg_open_read(HKEY_CURRENT_USER, r"Control Panel\Cursors") {
        let val = reg_get_string(hkey, "");
        unsafe { RegCloseKey(hkey) };
        if let Some(v) = val {
            if !v.trim().is_empty() {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

// Queries the current accessibility cursor size setting (1..16).
#[allow(dead_code)]
#[cfg(target_os = "windows")]
pub fn get_current_cursor_size() -> u32 {
    if let Some(hkey) = reg_open_read(HKEY_CURRENT_USER, r"Software\Microsoft\Accessibility") {
        let val = reg_get_dword(hkey, "CursorSize");
        unsafe { RegCloseKey(hkey) };
        if let Some(v) = val {
            return v.clamp(1, 16);
        }
    }
    1
}

// Expands Windows environment variables like %SYSTEMROOT% (e.g., C:\Windows\Cursors\aero_arrow.cur).
#[cfg(target_os = "windows")]
fn expand_env_path(path: &str) -> String {
    if !path.contains('%') {
        return path.to_string();
    }
    let wide_src: Vec<u16> = to_wide(path);
    let mut wide_dst = vec![0u16; 1024];
    let len = unsafe {
        ExpandEnvironmentStringsW(
            wide_src.as_ptr(),
            wide_dst.as_mut_ptr(),
            wide_dst.len() as u32,
        )
    };
    if len > 0 && (len as usize) <= wide_dst.len() {
        String::from_utf16_lossy(&wide_dst[..(len as usize - 1)])
    } else {
        path.to_string()
    }
}

// Reads all currently assigned cursor file paths from HKCU\Control Panel\Cursors.
#[cfg(target_os = "windows")]
fn get_active_cursors() -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(hkey) = reg_open_read(HKEY_CURRENT_USER, r"Control Panel\Cursors") {
        for &name in &CURSOR_NAMES {
            if let Some(path) = reg_get_string(hkey, name) {
                if !path.trim().is_empty() {
                    map.insert(name.to_lowercase(), path);
                }
            }
        }
        unsafe { RegCloseKey(hkey) };
    }
    map
}

// Loads and replaces system cursors dynamically at runtime with desired pixel dimensions.
// SetSystemCursor takes ownership of cursor handles and deletes them.
// File-based cursors are loaded with LR_LOADFROMFILE at exact dimensions.
// System built-in cursors loaded with LR_SHARED must be duplicated with CopyImage first.
#[cfg(target_os = "windows")]
pub fn apply_cursor_size(cursor_base_size: u32) -> bool {
    let cursors = get_active_cursors();

    for (name, ocr_id) in OCR_MAPPINGS {
        let path_opt = cursors
            .get(&name.to_lowercase())
            .map(|p| expand_env_path(p));

        let mut applied = false;
        if let Some(path) = path_opt {
            let trimmed = path.trim();
            if !trimmed.is_empty() && Path::new(trimmed).exists() {
                let wide = to_wide(trimmed);
                let hcur = unsafe {
                    LoadImageW(
                        std::ptr::null_mut(),
                        wide.as_ptr(),
                        IMAGE_CURSOR,
                        cursor_base_size as i32,
                        cursor_base_size as i32,
                        LR_LOADFROMFILE,
                    )
                };

                if !hcur.is_null() {
                    // SetSystemCursor takes ownership and destroys hcur on success.
                    let res = unsafe { SetSystemCursor(hcur, ocr_id) };
                    applied = res != 0;
                }
            }
        }

        if !applied {
            unsafe {
                let shared_cur = LoadImageW(
                    std::ptr::null_mut(),
                    ocr_id as usize as *const u16,
                    IMAGE_CURSOR,
                    cursor_base_size as i32,
                    cursor_base_size as i32,
                    LR_SHARED,
                );
                if !shared_cur.is_null() {
                    // Shared handles cannot be passed directly to SetSystemCursor (it would destroy them).
                    // Duplicate via CopyImage first.
                    let copy = CopyImage(
                        shared_cur,
                        IMAGE_CURSOR,
                        cursor_base_size as i32,
                        cursor_base_size as i32,
                        0,
                    );
                    if !copy.is_null() {
                        SetSystemCursor(copy, ocr_id);
                    }
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schemes_and_active() {
        let schemes = get_available_schemes();
        println!("Available schemes ({}):", schemes.len());
        for (i, s) in schemes.iter().enumerate() {
            println!("  [{i}] {s}");
        }
        let active = get_current_active_scheme();
        println!("Active scheme: {:?}", active);
        let size = get_current_cursor_size();
        println!("Current size: {size}");
    }
}
