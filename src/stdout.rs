use std::io::{self, Write};

pub fn write_png_to_stdout(png_bytes: &[u8]) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    if let Err(e) = stdout.write_all(png_bytes) {
        if e.kind() == io::ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(e);
    }
    if let Err(e) = stdout.flush() {
        if e.kind() == io::ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(e);
    }
    Ok(())
}
