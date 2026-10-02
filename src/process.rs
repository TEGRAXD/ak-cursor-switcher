// Checks if a process with matching executable name is currently active.
pub fn is_running(process_name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        is_running_win(process_name)
    }

    #[cfg(target_os = "linux")]
    {
        is_running_linux(process_name)
    }
}

// Windows implementation: Takes a process snapshot via Win32 Toolhelp32 API
// and iterates active processes using Process32FirstW / Process32NextW.
#[cfg(target_os = "windows")]
fn is_running_win(process_name: &str) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        // Create snapshot of all running processes in the system
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return false;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                // szExeFile is a null-terminated UTF-16 wchar array (u16)
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]);

                if name.eq_ignore_ascii_case(process_name) {
                    CloseHandle(snapshot);
                    return true;
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
        false
    }
}

// Linux implementation: Reads `/proc/[pid]/comm` to match process names.
#[cfg(target_os = "linux")]
fn is_running_linux(process_name: &str) -> bool {
    let entries = match std::fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();

        let pid = match file_name.to_str() {
            Some(name) => name,
            None => continue,
        };

        if !pid.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let comm_path = entry.path().join("comm");

        let process = match std::fs::read_to_string(comm_path) {
            Ok(process) => process,
            Err(_) => continue,
        };

        if process.trim().eq_ignore_ascii_case(process_name) {
            return true;
        }
    }

    false
}
