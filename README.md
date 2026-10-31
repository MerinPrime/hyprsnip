# HyprSnip

HyprSnip is a fast, predictable, and Unix-friendly screenshot utility designed specifically for the [Hyprland](https://hyprland.org) Wayland compositor. It features automatic window snapping, window corner rounding detection, and independent management of screenshot destinations (file storage, Wayland clipboard, and standard output stream).

---

## Features

- **Hyprland Window Snapping**: Automatically detects active Hyprland windows and snaps the selection rectangle to them.
- **Corner Rounding Preservation**: Queries Hyprland's `decoration:rounding` setting to crop windows with rounded corners cleanly.
- **Flexible Destination Management**:
  - Save to standard or custom directories with safe collision-free naming.
  - Save to specific `.png` files with overwrite protection.
  - Copy directly to the Wayland system clipboard.
  - Stream binary PNG bytes to `stdout` for Unix pipelines.
- **Single-Capture Pipeline**: Multiple destinations (e.g. file, clipboard, stdout) share a single screenshot capture without redundant screen grabs or re-encodes.
- **Fast Pre-Flight Validation**: Validates paths, directories, and write permissions before opening the overlay, failing immediately with descriptive errors if a target cannot be written.
- **Safe Atomic Writes**: Prevents partially written or empty files on disk if an error occurs.
- **Desktop Notifications**: Provides concise status messages and desktop notifications (with thumbnail support) when not in `--stdout` mode.

---

## CLI Usage

```text
Usage: hyprsnip [OPTIONS]

Options:
  -o, --output <PATH>            Path to save screenshot (directory or specific .png file)
  -c, --clipboard                Copy screenshot to system clipboard
      --stdout                   Output image to stdout in binary PNG format
  -a, --active                   Instantly capture active screen and copy to clipboard without overlay
  -f, --fullscreen               Instantly capture active screen without opening selection overlay
  -s, --screen-on-empty          Capture entire screen when clicking on empty desktop space
  -n, --no-notify                Disable desktop notifications
      --no-window                Disable window snapping (region selection only)
      --stroke-color <COLOR>     Overlay border stroke color (hex #RRGGBB or #RRGGBBAA) [default: #89b4faff]
      --dim-color <COLOR>        Overlay dimmed background color (hex #RRGGBB or #RRGGBBAA) [default: #0000008c]
      --inside-color <COLOR>     Overlay selection inside fill color (hex #RRGGBB or #RRGGBBAA) [default: #00000000]
      --no-rounding              Disable window corner rounding detection
  -h, --help                     Print help (and usage examples)
  -V, --version                  Print version
```

### Destination Behavior & Independence Matrix

All destination operations are fully independent and can be combined seamlessly:

| Command | Result |
|---|---|
| `hyprsnip` | Save to default directory (`~/Pictures/Screenshots/`) |
| `hyprsnip -o ~/Screenshots` | Save to specified directory with auto-generated filename |
| `hyprsnip -o ~/Screenshots/my_shot.png` | Save to a specific file |
| `hyprsnip -c` | Only copy to clipboard (no disk file saved) |
| `hyprsnip -o ~/Screenshots -c` | Save to directory AND copy to clipboard |
| `hyprsnip --stdout` | Only output binary PNG to stdout |
| `hyprsnip -o ~/Screenshots -c --stdout` | Save, copy to clipboard, AND pipe to stdout |

> **Note**: If `--clipboard` or `--stdout` is explicitly requested without `--output`, HyprSnip will **not** save an extra copy to the default directory.

---

## Common Scenarios & Examples

### 1. Instant Active Screen Copy
Instantly captures the focused/active monitor and copies it to the clipboard without opening the selection overlay:
```bash
hyprsnip -a
# or
hyprsnip -fc
```

### 2. Default Screenshot
Takes a screenshot and saves it to `~/Pictures/Screenshots/` with an auto-generated timestamp filename (e.g. `hyprsnip_2026-10-31_15-30-12.png`):
```bash
hyprsnip
```
If the directory does not exist, it is created automatically. If a filename collision occurs, HyprSnip safely appends `_1.png`, `_2.png`, etc., without overwriting any existing files.

### 2. Save to Another Directory
```bash
hyprsnip -o ~/Screenshots
# or with trailing slash
hyprsnip --output ~/Screenshots/
```
If the directory does not exist, HyprSnip creates all missing parent directories.

### 3. Save to a Specific File
```bash
hyprsnip -o ~/Screenshots/my_screenshot.png
```
HyprSnip saves the screenshot directly under the specified name. If the file already exists, HyprSnip refuses to overwrite it silently and aborts with an informative error message.

### 4. Clipboard Only
Copies the screenshot directly to the Wayland system clipboard as `image/png`:
```bash
hyprsnip -c
# or
hyprsnip --clipboard
```
No file is created on disk. Requires `wl-clipboard` (`wl-copy`).

### 5. Simultaneous Save and Copy
Saves to disk and copies to the clipboard in a single execution:
```bash
hyprsnip -o ~/Screenshots/my_screenshot.png -c
```
The screen is captured exactly once, cropped once, and used for both operations.

### 6. Disabling Desktop Notifications
Use `-n` or `--no-notify` (alias `--no-notification`) to suppress desktop notifications:
```bash
# Copy to clipboard without showing a desktop notification
hyprsnip -cn

# Fullscreen capture without notifications
hyprsnip -fn
```

### 7. Stdout and Unix Pipelines
Outputs the raw PNG bytes directly to `stdout`:
```bash
# Pipe directly to external tools or clipboard
hyprsnip --stdout | wl-copy

# Pipe into an image processing tool (e.g. ImageMagick)
hyprsnip --stdout | magick - -resize 50% /tmp/thumbnail.png
```
In `--stdout` mode:
- Only binary image data is written to standard output.
- All diagnostic and error messages are sent to `stderr`.
- Desktop notifications and terminal status messages are suppressed to ensure pipelines are clean.
- Broken pipe conditions (e.g. piping into `head`) are handled gracefully without panicking.

---

## Path Resolution Rules

When specifying `-o, --output <PATH>`:
1. **Tilde expansion**: Paths starting with `~` or `~/` are automatically expanded to the user's `$HOME` directory.
2. **Directory Mode**:
   - Any path ending in `/` or `\` is treated as a directory.
   - Any existing path that is a directory is treated as a directory.
   - Any non-existing path that does **not** end in `.png` (e.g. `hyprsnip -o ~/Screenshots`) is treated as a directory and created automatically. Auto-generated timestamped files (`hyprsnip_YYYY-MM-DD_HH-MM-SS.png`) will be stored inside.
3. **File Mode**:
   - Any path ending in `.png` (case-insensitive, e.g. `.png`, `.PNG`) is treated as a concrete file path.
   - Any existing path that is a regular file is treated as a concrete file path.
4. **Collision and Overwrite Protection**:
   - In **File Mode**: HyprSnip will never silently overwrite an existing file; it aborts with an error before capture.
   - In **Directory Mode**: HyprSnip safely generates unique names (`hyprsnip_YYYY-MM-DD_HH-MM-SS_1.png`) if a collision occurs.
5. **Atomic Writing**:
   - Files are written safely with temporary files and atomic linking/renaming. In case of write failure (e.g., out of disk space), no empty or partially written files are left on disk.

---

## Dependencies & Requirements

### Runtime Dependencies
- `hyprland`: Wayland compositor
- `wayland`: Core Wayland protocol libraries
- `libxkbcommon`: Keyboard state handling
- `wl-clipboard` (*optional, recommended*): Required for clipboard support (`-c, --clipboard`)
- `libnotify` (*optional*): Required for desktop notifications (`notify-send`)

### Build Dependencies
- `cargo` / `rustc` (edition 2021)

---

## Building and Installation

### From Source
```bash
git clone https://github.com/MerinPrime/hyprsnip.git
cd hyprsnip
cargo build --release
sudo install -Dm755 target/release/hyprsnip /usr/local/bin/hyprsnip
```

### Arch Linux (PKGBUILD)
A `PKGBUILD` is provided in the repository root:
```bash
makepkg -si
```
