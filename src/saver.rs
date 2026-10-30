use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::destination::{generate_timestamp, DestinationMode};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_counter() -> usize {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn save_screenshot(mode: &DestinationMode, png_bytes: &[u8]) -> Result<PathBuf, String> {
    match mode {
        DestinationMode::Directory(dir) => save_to_directory(dir, png_bytes),
        DestinationMode::File(file) => save_to_specific_file(file, png_bytes),
    }
}

fn save_to_directory(dir: &Path, png_bytes: &[u8]) -> Result<PathBuf, String> {
    if !dir.exists() {
        fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create directory '{}': {}", dir.display(), e))?;
    }

    let ts = generate_timestamp();
    let base_name = format!("hyprsnip_{ts}.png");
    let first_target = dir.join(&base_name);

    if !first_target.exists() {
        if let Ok(path) = try_write_new_file(&first_target, png_bytes) {
            return Ok(path);
        }
    }

    for counter in 1..=10000 {
        let candidate_name = format!("hyprsnip_{ts}_{counter}.png");
        let candidate_path = dir.join(candidate_name);
        if !candidate_path.exists() {
            if let Ok(path) = try_write_new_file(&candidate_path, png_bytes) {
                return Ok(path);
            }
        }
    }

    Err(format!(
        "Failed to find unique filename in directory '{}'",
        dir.display()
    ))
}

fn save_to_specific_file(target: &Path, png_bytes: &[u8]) -> Result<PathBuf, String> {
    if target.exists() {
        return Err(format!(
            "Target file '{}' already exists. Refusing to overwrite existing file.",
            target.display()
        ));
    }
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create parent directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }
    }

    try_write_new_file(target, png_bytes)
}

fn try_write_new_file(target: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let temp_name = format!(
        ".hyprsnip_tmp_{}_{}_{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
        next_counter()
    );
    let temp_path = parent.join(temp_name);

    let write_res = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_res {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("Failed to write screenshot data: {}", e));
    }

    let link_res = fs::hard_link(&temp_path, target);
    let _ = fs::remove_file(&temp_path);

    match link_res {
        Ok(_) => Ok(target.to_path_buf()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(format!("Target file '{}' already exists", target.display()))
        }
        Err(_) => {
            if target.exists() {
                return Err(format!("Target file '{}' already exists", target.display()));
            }
            fs::rename(&temp_path, target).map_err(|e| {
                let _ = fs::remove_file(&temp_path);
                format!("Failed to finalize file '{}': {}", target.display(), e)
            })?;
            Ok(target.to_path_buf())
        }
    }
}
