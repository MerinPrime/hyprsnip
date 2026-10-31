use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone, PartialEq, Eq)]
#[command(
    name = "hyprsnip",
    author,
    version,
    about = "Hyprland screenshot tool",
    long_about = "HyprSnip is a fast, predictable screenshot tool designed for Hyprland.\n\
                  It allows independent control over saving to disk, copying to clipboard,\n\
                  and streaming binary image data to stdout.",
    after_help = "EXAMPLES:\n  \
        hyprsnip\n      \
            Take a screenshot and save to ~/Pictures/Screenshots/hyprsnip_<timestamp>.png\n\n  \
        hyprsnip -a\n      \
            Instantly copy active screen to clipboard without opening selection overlay\n\n  \
        hyprsnip -fc\n      \
            Instantly capture active screen and copy to clipboard\n\n  \
        hyprsnip -f\n      \
            Instantly capture active screen and save to default directory\n\n  \
        hyprsnip -s\n      \
            Allow capturing entire screen on empty space click (toggleable with Space key)\n\n  \
        hyprsnip -o ~/Screenshots\n      \
            Save screenshot to the specified directory with auto-generated filename\n\n  \
        hyprsnip -o ~/Screenshots/my_screenshot.png\n      \
            Save screenshot to a specific file\n\n  \
        hyprsnip -c\n  \
        hyprsnip --clipboard\n      \
            Copy screenshot directly to system clipboard (no file saved)\n\n  \
        hyprsnip -cn\n      \
            Copy screenshot directly to system clipboard without notifications\n\n  \
        hyprsnip -o ~/Screenshots/test.png -c\n      \
            Save to file and copy to clipboard simultaneously\n\n  \
        hyprsnip --stdout | wl-copy\n      \
            Output raw PNG bytes to stdout (for Unix pipelines)\n\n  \
        hyprsnip -o ~/Screenshots -c --stdout\n      \
            Save to directory, copy to clipboard, and pipe to stdout"
)]
pub struct Cli {
    #[arg(
        short = 'o',
        long = "output",
        value_name = "PATH",
        help = "Path to save screenshot (directory or specific .png file)"
    )]
    pub output: Option<PathBuf>,

    #[arg(
        short = 'c',
        long = "clipboard",
        help = "Copy screenshot to system clipboard"
    )]
    pub clipboard: bool,

    #[arg(long = "stdout", help = "Output image to stdout in binary PNG format")]
    pub stdout: bool,

    #[arg(
        short = 'f',
        long = "fullscreen",
        alias = "screen",
        help = "Instantly capture active screen without opening selection overlay"
    )]
    pub fullscreen: bool,

    #[arg(
        short = 'a',
        long = "active",
        help = "Instantly capture active screen and copy to clipboard without overlay"
    )]
    pub active: bool,

    #[arg(
        short = 's',
        long = "screen-on-empty",
        help = "Capture entire screen when clicking on empty desktop space"
    )]
    pub screen_on_empty: bool,

    #[arg(
        short = 'n',
        long = "no-notify",
        alias = "no-notification",
        help = "Disable desktop notifications"
    )]
    pub no_notify: bool,

    #[arg(long, help = "Disable window snapping (region selection only)")]
    pub no_window: bool,

    #[arg(
        long,
        default_value = "#89b4faff",
        help = "Overlay border stroke color (hex #RRGGBB or #RRGGBBAA)"
    )]
    pub stroke_color: String,

    #[arg(
        long,
        default_value = "#0000008c",
        help = "Overlay dimmed background color (hex #RRGGBB or #RRGGBBAA)"
    )]
    pub dim_color: String,

    #[arg(
        long,
        default_value = "#00000000",
        help = "Overlay selection inside fill color (hex #RRGGBB or #RRGGBBAA)"
    )]
    pub inside_color: String,

    #[arg(
        long,
        default_value_t = false,
        help = "Disable window corner rounding detection"
    )]
    pub no_rounding: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionPlan {
    pub save_path: Option<PathBuf>,
    pub clipboard: bool,
    pub stdout: bool,
    pub notify: bool,
}

impl Cli {
    pub fn plan_actions(&self) -> ActionPlan {
        let clipboard = self.clipboard || self.active;
        let should_save = if self.output.is_some() {
            true
        } else {
            !clipboard && !self.stdout
        };

        ActionPlan {
            save_path: if should_save {
                self.output
                    .clone()
                    .or_else(|| Some(PathBuf::from("~/Pictures/Screenshots/")))
            } else {
                None
            },
            clipboard,
            stdout: self.stdout,
            notify: !self.no_notify && !self.stdout,
        }
    }
}
