use hyprsnip::destination::DestinationMode;
use hyprsnip::saver::save_screenshot;
use std::fs;
use std::path::PathBuf;

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
fn test_save_to_directory_creates_timestamped_file() {
    let test_dir = TestDir::new("save_dir");
    let fake_png = b"\x89PNG\r\n\x1a\nfake png data";

    let mode = DestinationMode::Directory(test_dir.path.clone());
    let saved_path = save_screenshot(&mode, fake_png).expect("save_screenshot failed");

    assert!(saved_path.exists());
    assert_eq!(fs::read(&saved_path).unwrap(), fake_png);

    let file_name = saved_path.file_name().unwrap().to_string_lossy();
    assert!(file_name.starts_with("hyprsnip_"));
    assert!(file_name.ends_with(".png"));
}

#[test]
fn test_save_to_directory_collision_creates_unique_name() {
    let test_dir = TestDir::new("save_collision");
    let fake_png1 = b"\x89PNG\r\n\x1a\ncontent 1";
    let fake_png2 = b"\x89PNG\r\n\x1a\ncontent 2";

    let mode = DestinationMode::Directory(test_dir.path.clone());
    let saved1 = save_screenshot(&mode, fake_png1).expect("first save failed");
    let saved2 = save_screenshot(&mode, fake_png2).expect("second save failed");

    assert_ne!(saved1, saved2);
    assert!(saved1.exists());
    assert!(saved2.exists());

    assert_eq!(fs::read(&saved1).unwrap(), fake_png1);
    assert_eq!(fs::read(&saved2).unwrap(), fake_png2);
}

#[test]
fn test_save_to_specific_file() {
    let test_dir = TestDir::new("save_file");
    let target_file = test_dir.path.join("custom_screenshot.png");
    let fake_png = b"\x89PNG\r\n\x1a\ncustom content";

    let mode = DestinationMode::File(target_file.clone());
    let saved_path = save_screenshot(&mode, fake_png).expect("save_screenshot failed");

    assert_eq!(saved_path, target_file);
    assert!(target_file.exists());
    assert_eq!(fs::read(&target_file).unwrap(), fake_png);
}

#[test]
fn test_save_to_specific_file_refuses_to_overwrite_existing() {
    let test_dir = TestDir::new("save_no_overwrite");
    let target_file = test_dir.path.join("existing.png");
    let original_content = b"original important data";
    fs::write(&target_file, original_content).unwrap();

    let new_png = b"\x89PNG\r\n\x1a\nnew data";
    let mode = DestinationMode::File(target_file.clone());
    let result = save_screenshot(&mode, new_png);

    assert!(result.is_err(), "Expected error when file already exists");

    assert_eq!(fs::read(&target_file).unwrap(), original_content);
}

#[test]
fn test_save_to_nested_uncreated_directory() {
    let test_dir = TestDir::new("save_nested");
    let target_file = test_dir.path.join("a/b/c/d/my_shot.png");
    let fake_png = b"\x89PNG\r\n\x1a\nnested content";

    let mode = DestinationMode::File(target_file.clone());
    let saved = save_screenshot(&mode, fake_png).expect("save failed");

    assert_eq!(saved, target_file);
    assert!(target_file.exists());
    assert_eq!(fs::read(&target_file).unwrap(), fake_png);
}
