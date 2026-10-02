use std::fs;
use std::path::{Path, PathBuf};

pub const ASSET_FILENAME: &str = "a9d41799f1af1868f2db495671227cd4.bin";
pub const ASSET_FILENAME_DISABLED: &str = "a9d41799f1af1868f2db495671227cd4.bin.disabled";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixStatus {
    /// In-game software cursor is active (.bin exists). Causes mouse stutter.
    ActiveSoftwareCursor,
    /// Hardware cursor fix applied (.bin.disabled exists). Smooth mouse, no stutter.
    FixedDisabled,
    /// Neither file found at detected or given path.
    NotFound,
}

/// Recursively looks for the asset bundle file within a directory up to a max depth.
fn find_file_recursive(dir: &Path, depth: usize) -> Option<PathBuf> {
    if depth == 0 || !dir.is_dir() {
        return None;
    }
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.eq_ignore_ascii_case(ASSET_FILENAME)
                    || name.eq_ignore_ascii_case(ASSET_FILENAME_DISABLED)
                {
                    return Some(path);
                }
            }
        } else if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

/// Attempts to locate the Arknights cursor asset bundle file:
/// 1. Direct path check from custom/saved path (file or directory).
/// 2. If given an executable (e.g. Arknights.exe), checks its parent folder and subdirectories.
pub fn locate_asset_file(custom_path: Option<&str>) -> Option<PathBuf> {
    // 1. Check custom configured path
    if let Some(p_str) = custom_path {
        let trimmed = p_str.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.is_file() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if name.eq_ignore_ascii_case(ASSET_FILENAME)
                        || name.eq_ignore_ascii_case(ASSET_FILENAME_DISABLED)
                    {
                        return Some(p);
                    }
                }
                // If an executable (or other file) was selected, inspect its parent directory
                if let Some(parent) = p.parent() {
                    let direct_paths = [
                        parent
                            .join(r"Arknights_Data\StreamingAssets\AB\Windows\anon")
                            .join(ASSET_FILENAME),
                        parent
                            .join(r"Arknights_Data\StreamingAssets\AB\Windows\anon")
                            .join(ASSET_FILENAME_DISABLED),
                        parent
                            .join(r"Arknights_EN\Arknights_Data\StreamingAssets\AB\Windows\anon")
                            .join(ASSET_FILENAME),
                        parent
                            .join(r"Arknights_EN\Arknights_Data\StreamingAssets\AB\Windows\anon")
                            .join(ASSET_FILENAME_DISABLED),
                        parent
                            .join(r"StreamingAssets\AB\Windows\anon")
                            .join(ASSET_FILENAME),
                        parent
                            .join(r"StreamingAssets\AB\Windows\anon")
                            .join(ASSET_FILENAME_DISABLED),
                    ];
                    for candidate in direct_paths {
                        if candidate.exists() {
                            return Some(candidate);
                        }
                    }
                    if let Some(found) = find_file_recursive(parent, 6) {
                        return Some(found);
                    }
                }
            } else if p.is_dir() {
                let direct_paths = [
                    p.join(r"Arknights_Data\StreamingAssets\AB\Windows\anon")
                        .join(ASSET_FILENAME),
                    p.join(r"Arknights_Data\StreamingAssets\AB\Windows\anon")
                        .join(ASSET_FILENAME_DISABLED),
                    p.join(r"Arknights_EN\Arknights_Data\StreamingAssets\AB\Windows\anon")
                        .join(ASSET_FILENAME),
                    p.join(r"Arknights_EN\Arknights_Data\StreamingAssets\AB\Windows\anon")
                        .join(ASSET_FILENAME_DISABLED),
                    p.join(r"StreamingAssets\AB\Windows\anon")
                        .join(ASSET_FILENAME),
                    p.join(r"StreamingAssets\AB\Windows\anon")
                        .join(ASSET_FILENAME_DISABLED),
                ];
                for candidate in direct_paths {
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
                if let Some(found) = find_file_recursive(&p, 6) {
                    return Some(found);
                }
            }
        }
    }

    None
}

/// Checks the current status of the asset file at the given path.
pub fn get_status_of_file(file_path: &Path) -> FixStatus {
    if !file_path.exists() {
        return FixStatus::NotFound;
    }
    if let Some(name) = file_path.file_name().and_then(|n| n.to_str()) {
        if name.eq_ignore_ascii_case(ASSET_FILENAME) {
            return FixStatus::ActiveSoftwareCursor;
        }
        if name.eq_ignore_ascii_case(ASSET_FILENAME_DISABLED) {
            return FixStatus::FixedDisabled;
        }
    }
    FixStatus::NotFound
}

/// Renames the file to toggle between Active (.bin) and Fixed (.bin.disabled).
/// Returns the new status and new path on success.
pub fn toggle_fix(current_path: &Path) -> Result<(FixStatus, PathBuf), String> {
    if !current_path.exists() {
        return Err("Target file does not exist.".to_string());
    }

    let parent = current_path
        .parent()
        .ok_or_else(|| "Failed to get parent directory.".to_string())?;

    let file_name = current_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid file name.".to_string())?;

    if file_name.eq_ignore_ascii_case(ASSET_FILENAME) {
        // Disable software cursor -> rename to .bin.disabled
        let target_path = parent.join(ASSET_FILENAME_DISABLED);
        if target_path.exists() {
            let _ = fs::remove_file(&target_path);
        }
        fs::rename(current_path, &target_path)
            .map_err(|e| format!("Failed to rename file to .disabled: {e}"))?;
        Ok((FixStatus::FixedDisabled, target_path))
    } else if file_name.eq_ignore_ascii_case(ASSET_FILENAME_DISABLED) {
        // Re-enable software cursor -> rename back to .bin
        let target_path = parent.join(ASSET_FILENAME);
        if target_path.exists() {
            let _ = fs::remove_file(&target_path);
        }
        fs::rename(current_path, &target_path)
            .map_err(|e| format!("Failed to restore file to .bin: {e}"))?;
        Ok((FixStatus::ActiveSoftwareCursor, target_path))
    } else {
        Err(format!(
            "File '{file_name}' is not recognized as the cursor asset bundle."
        ))
    }
}
