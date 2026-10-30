use hyprsnip::destination::{
    check_directory_writable, determine_destination_mode, expand_tilde,
    validate_destination_preflight, validate_file_name, DestinationMode,
};
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
fn test_expand_tilde() {
    let home = std::env::var_os("HOME").expect("HOME not set");
    let home_path = PathBuf::from(&home);

    assert_eq!(expand_tilde(Path::new("~")), home_path);
    assert_eq!(
        expand_tilde(Path::new("~/Pictures/Screenshots")),
        home_path.join("Pictures/Screenshots")
    );
    assert_eq!(
        expand_tilde(Path::new("/var/log/test.png")),
        PathBuf::from("/var/log/test.png")
    );
    assert_eq!(
        expand_tilde(Path::new("relative/path/test.png")),
        PathBuf::from("relative/path/test.png")
    );
}

#[test]
fn test_determine_mode_trailing_slash() {
    let mode = determine_destination_mode("some_dir/", Path::new("/tmp/some_dir"));
    assert_eq!(
        mode,
        DestinationMode::Directory(PathBuf::from("/tmp/some_dir"))
    );
}

#[test]
fn test_determine_mode_ends_with_png() {
    let mode = determine_destination_mode("shot.png", Path::new("/tmp/shot.png"));
    assert_eq!(mode, DestinationMode::File(PathBuf::from("/tmp/shot.png")));

    let mode_upper = determine_destination_mode("shot.PNG", Path::new("/tmp/shot.PNG"));
    assert_eq!(
        mode_upper,
        DestinationMode::File(PathBuf::from("/tmp/shot.PNG"))
    );
}

#[test]
fn test_determine_mode_existing_dir_and_file() {
    let test_dir = TestDir::new("mode_existing");
    let existing_dir = test_dir.path.join("subdir");
    fs::create_dir_all(&existing_dir).unwrap();

    let existing_file = test_dir.path.join("regular_file.dat");
    fs::write(&existing_file, b"data").unwrap();

    let mode_dir = determine_destination_mode(&existing_dir.to_string_lossy(), &existing_dir);
    assert_eq!(mode_dir, DestinationMode::Directory(existing_dir));

    let mode_file = determine_destination_mode(&existing_file.to_string_lossy(), &existing_file);
    assert_eq!(mode_file, DestinationMode::File(existing_file));
}

#[test]
fn test_determine_mode_ambiguous_nonexistent_treated_as_directory() {
    let mode = determine_destination_mode(
        "/tmp/hyprsnip_nonexistent_dir",
        Path::new("/tmp/hyprsnip_nonexistent_dir"),
    );
    assert_eq!(
        mode,
        DestinationMode::Directory(PathBuf::from("/tmp/hyprsnip_nonexistent_dir"))
    );

    let mode_dots =
        determine_destination_mode("/tmp/dir.with.dots", Path::new("/tmp/dir.with.dots"));
    assert_eq!(
        mode_dots,
        DestinationMode::Directory(PathBuf::from("/tmp/dir.with.dots"))
    );
}

#[test]
fn test_validate_filename() {
    assert!(validate_file_name(Path::new("my_shot.png")).is_ok());
    assert!(validate_file_name(Path::new("/tmp/some_dir/my_shot.png")).is_ok());

    assert!(validate_file_name(Path::new("")).is_err());
    assert!(validate_file_name(Path::new(".")).is_err());
    assert!(validate_file_name(Path::new("..")).is_err());

    let long_name = format!("{}.png", "a".repeat(260));
    assert!(validate_file_name(Path::new(&long_name)).is_err());
}

#[test]
fn test_check_directory_writable() {
    let test_dir = TestDir::new("writable");
    assert!(check_directory_writable(&test_dir.path).is_ok());

    let nonexistent = test_dir.path.join("does_not_exist");
    assert!(check_directory_writable(&nonexistent).is_err());
}

#[test]
fn test_preflight_prevents_silent_overwrite_of_existing_file() {
    let test_dir = TestDir::new("preflight_overwrite");
    let existing_file = test_dir.path.join("existing.png");
    fs::write(&existing_file, b"existing content").unwrap();

    let mode = DestinationMode::File(existing_file);
    let res = validate_destination_preflight(&mode);
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    assert!(
        err_msg.contains("already exists"),
        "Expected error message about existing file, got: {}",
        err_msg
    );
}

#[test]
fn test_preflight_creates_missing_parent_directories() {
    let test_dir = TestDir::new("preflight_create");
    let nested_file = test_dir.path.join("nested/level1/level2/screenshot.png");

    let mode = DestinationMode::File(nested_file.clone());
    assert!(validate_destination_preflight(&mode).is_ok());
    assert!(nested_file.parent().unwrap().exists());
}
