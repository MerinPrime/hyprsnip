use std::path::Path;
use std::process::Command;

pub fn send_notification(summary: &str, body: &str, icon_path: Option<&Path>) {
    let mut cmd = Command::new("notify-send");
    cmd.arg("-a").arg("HyprSnip");
    if let Some(icon) = icon_path {
        cmd.arg("-i").arg(icon);
    }
    cmd.arg(summary).arg(body);
    let _ = cmd.spawn();
}
