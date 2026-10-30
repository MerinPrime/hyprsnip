use hyprsnip::clipboard::copy_to_clipboard;
use std::process::Command;

#[test]
fn test_clipboard_copy_success_if_wayland_present() {
    if std::env::var("WAYLAND_DISPLAY").is_err() {
        eprintln!("Skipping clipboard test: WAYLAND_DISPLAY not set");
        return;
    }

    let fake_png = b"\x89PNG\r\n\x1a\nclipboard test data";
    let res = copy_to_clipboard(fake_png);
    if let Err(e) = &res {
        eprintln!("copy_to_clipboard returned error: {e}");
    }
    assert!(res.is_ok(), "copy_to_clipboard should succeed on Wayland");

    let output = Command::new("wl-paste")
        .arg("-l")
        .output()
        .expect("wl-paste failed to execute");
    let types = String::from_utf8_lossy(&output.stdout);
    assert!(
        types.contains("image/png"),
        "wl-paste types should contain image/png, got: {types}"
    );
}
