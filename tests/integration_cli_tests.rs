use hyprsnip::clipboard::ClipboardError;
use hyprsnip::destination::{
    determine_destination_mode, expand_tilde, validate_destination_preflight, DestinationMode,
};
use hyprsnip::saver::save_screenshot;
use hyprsnip::stdout::write_png_to_stdout;
use std::fs;
use std::path::{Path, PathBuf};

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("hyprsnip_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create test dir");
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_png_binary_magic_header_integrity() {
    let png_header = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let test_dir = TestDir::new("png_magic");
    let target = test_dir.path.join("shot.png");

    let mode = DestinationMode::File(target.clone());
    save_screenshot(&mode, png_header).expect("Save failed");

    let saved_data = fs::read(&target).expect("Read failed");
    assert_eq!(&saved_data[0..8], png_header);
}

#[test]
fn test_relative_paths() {
    let test_dir = TestDir::new("relative");
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&test_dir.path).unwrap();

    let rel_file_path = PathBuf::from("relative_shot.png");
    let mode_file = determine_destination_mode(
        &rel_file_path.to_string_lossy(),
        &expand_tilde(&rel_file_path),
    );
    assert_eq!(mode_file, DestinationMode::File(rel_file_path.clone()));

    assert!(validate_destination_preflight(&mode_file).is_ok());

    let png_data = b"\x89PNG\r\n\x1a\nrelative test";
    let saved = save_screenshot(&mode_file, png_data).expect("Save failed");
    assert_eq!(saved, rel_file_path);
    assert!(rel_file_path.exists());
    assert_eq!(fs::read(&rel_file_path).unwrap(), png_data);

    let rel_dir_path = PathBuf::from("rel_dir/");
    let mode_dir = determine_destination_mode(
        &rel_dir_path.to_string_lossy(),
        &expand_tilde(&rel_dir_path),
    );
    assert_eq!(mode_dir, DestinationMode::Directory(rel_dir_path.clone()));
    let saved_dir_file = save_screenshot(&mode_dir, png_data).expect("Save in rel dir failed");
    assert!(saved_dir_file.exists());
    assert!(saved_dir_file.starts_with("rel_dir"));

    std::env::set_current_dir(old_cwd).unwrap();
}

#[test]
fn test_unwritable_directory_error() {
    let forbidden_dir = Path::new("/proc/hyprsnip_forbidden");
    let mode = DestinationMode::Directory(forbidden_dir.to_path_buf());
    let res = validate_destination_preflight(&mode);
    assert!(
        res.is_err(),
        "Expected preflight check to fail for non-writable system path"
    );
}

#[test]
fn test_missing_clipboard_tool_handling() {
    let err = ClipboardError::ToolNotFound;
    let msg = err.to_string();
    assert!(msg.contains("wl-copy"));
    assert!(msg.contains("wl-clipboard"));
}

#[test]
fn test_stdout_broken_pipe_handling() {
    let data = b"\x89PNG\r\n\x1a\nsmall buffer";
    let res = write_png_to_stdout(data);
    assert!(res.is_ok());
}
