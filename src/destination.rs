use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestinationMode {
    Directory(PathBuf),
    File(PathBuf),
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    } else if let Some(stripped) = s.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    path.to_path_buf()
}

pub fn determine_destination_mode(raw_path_str: &str, expanded_path: &Path) -> DestinationMode {
    if raw_path_str.ends_with('/') || raw_path_str.ends_with(std::path::MAIN_SEPARATOR) {
        return DestinationMode::Directory(expanded_path.to_path_buf());
    }

    if expanded_path.exists() {
        if expanded_path.is_dir() {
            return DestinationMode::Directory(expanded_path.to_path_buf());
        } else {
            return DestinationMode::File(expanded_path.to_path_buf());
        }
    }

    let has_png_ext = expanded_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("png"))
        .unwrap_or(false);

    if has_png_ext {
        DestinationMode::File(expanded_path.to_path_buf())
    } else {
        DestinationMode::Directory(expanded_path.to_path_buf())
    }
}

pub fn validate_file_name(path: &Path) -> Result<(), String> {
    let file_name = match path.file_name() {
        Some(name) => name,
        None => {
            return Err(format!(
                "Invalid file path '{}': missing filename",
                path.display()
            ))
        }
    };
    let file_name_str = file_name.to_string_lossy();
    if file_name_str.is_empty() || file_name_str == "." || file_name_str == ".." {
        return Err(format!("Invalid file name '{}'", file_name_str));
    }
    if file_name.len() > 255 {
        return Err(format!(
            "File name '{}' exceeds maximum allowed length of 255 bytes",
            file_name_str
        ));
    }
    if file_name_str.contains('\0') {
        return Err("File name contains invalid null byte".to_string());
    }
    Ok(())
}

pub fn check_directory_writable(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Err(format!("Directory '{}' does not exist", dir.display()));
    }
    if !dir.is_dir() {
        return Err(format!("'{}' is not a directory", dir.display()));
    }

    let c_path = CString::new(dir.as_os_str().as_bytes())
        .map_err(|_| "Directory path contains invalid null byte".to_string())?;

    let res = unsafe { libc::access(c_path.as_ptr(), libc::W_OK) };
    if res != 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!(
            "Directory '{}' is not writable: {}",
            dir.display(),
            err
        ));
    }
    Ok(())
}

pub fn validate_destination_preflight(mode: &DestinationMode) -> Result<(), String> {
    match mode {
        DestinationMode::Directory(dir) => {
            if !dir.exists() {
                std::fs::create_dir_all(dir).map_err(|e| {
                    format!(
                        "Failed to create destination directory '{}': {}",
                        dir.display(),
                        e
                    )
                })?;
            }
            check_directory_writable(dir)?;
        }
        DestinationMode::File(file) => {
            validate_file_name(file)?;
            if file.exists() {
                return Err(format!(
                    "Target file '{}' already exists. Refusing to overwrite existing file.",
                    file.display()
                ));
            }
            if let Some(parent) = file.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        format!(
                            "Failed to create parent directory '{}': {}",
                            parent.display(),
                            e
                        )
                    })?;
                }
                let check_dir = if parent.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    parent
                };
                check_directory_writable(check_dir)?;
            }
        }
    }
    Ok(())
}

pub fn generate_timestamp() -> String {
    let mut now: libc::time_t = 0;
    unsafe { libc::time(&mut now) };
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let res = unsafe { libc::localtime_r(&now, &mut tm) };
    if !res.is_null() {
        format!(
            "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    } else {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("timestamp_{}", d.as_secs())
    }
}
