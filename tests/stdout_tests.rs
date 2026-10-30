use hyprsnip::stdout::write_png_to_stdout;

#[test]
fn test_write_png_to_stdout() {
    let fake_png = b"\x89PNG\r\n\x1a\nstdout test bytes";

    let res = write_png_to_stdout(fake_png);
    assert!(res.is_ok());
}
