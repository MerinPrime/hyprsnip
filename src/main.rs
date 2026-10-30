use std::collections::HashSet;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use clap::Parser;
use hyprland::data::Clients;
use hyprland::keyword::OptionValue;
use hyprland::shared::HyprData;
use image::RgbaImage;
use libwayshot::WayshotConnection;

use hyprsnip::cli::Cli;
use hyprsnip::clipboard::copy_to_clipboard;
use hyprsnip::destination::{
    determine_destination_mode, expand_tilde, validate_destination_preflight,
};
use hyprsnip::notify::send_notification;
use hyprsnip::overlay::{
    crop_and_round, parse_hex_color, run_wayland_overlay, ParsedColors, Rect, SelectionState,
};
use hyprsnip::saver::save_screenshot;
use hyprsnip::stdout::write_png_to_stdout;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let plan = cli.plan_actions();

    let resolved_mode = if let Some(ref raw_path) = plan.save_path {
        let raw_str = raw_path.to_string_lossy();
        let expanded = expand_tilde(raw_path);
        let mode = determine_destination_mode(&raw_str, &expanded);
        validate_destination_preflight(&mode)?;
        Some(mode)
    } else {
        None
    };

    let colors = ParsedColors {
        stroke: parse_hex_color(&cli.stroke_color),
        dim: parse_hex_color(&cli.dim_color),
        inside: parse_hex_color(&cli.inside_color),
    };

    let wayshot = WayshotConnection::new()?;
    let screenshot: RgbaImage = wayshot.screenshot_all(false)?.to_rgba8();
    let (screen_w, screen_h) = (screenshot.width(), screenshot.height());
    let screenshot = Arc::new(screenshot);

    let rounding = if cli.no_rounding {
        0.0
    } else {
        hyprland::keyword::Keyword::get("decoration:rounding")
            .ok()
            .and_then(|kw| match kw.value {
                OptionValue::Int(v) => Some(v as f32),
                OptionValue::Float(v) => Some(v as f32),
                _ => None,
            })
            .unwrap_or(10.0)
    };

    let (target_rect, is_window) = if cli.fullscreen || cli.active {
        let monitors = hyprland::data::Monitors::get().ok();
        let cursor_pos = hyprland::data::CursorPosition::get().ok();

        let (min_x, min_y) = if let Some(ref mons) = monitors {
            (
                mons.iter().map(|m| m.x).min().unwrap_or(0),
                mons.iter().map(|m| m.y).min().unwrap_or(0),
            )
        } else {
            (0, 0)
        };

        let active_mon = if let Some(ref mons) = monitors {
            mons.iter()
                .find(|m| m.focused)
                .or_else(|| {
                    if let Some(pos) = cursor_pos {
                        mons.iter().find(|m| {
                            pos.x >= m.x as i64
                                && pos.x < (m.x + m.width as i32) as i64
                                && pos.y >= m.y as i64
                                && pos.y < (m.y + m.height as i32) as i64
                        })
                    } else {
                        None
                    }
                })
                .or_else(|| mons.iter().next())
        } else {
            None
        };

        let rect = if let Some(m) = active_mon {
            Rect {
                x: (m.x - min_x) as f32,
                y: (m.y - min_y) as f32,
                w: m.width as f32,
                h: m.height as f32,
            }
        } else {
            Rect {
                x: 0.0,
                y: 0.0,
                w: screen_w as f32,
                h: screen_h as f32,
            }
        };

        (rect, false)
    } else {
        let mut active_workspaces = HashSet::new();
        if let Ok(monitors) = hyprland::data::Monitors::get() {
            for m in monitors {
                active_workspaces.insert(m.active_workspace.id);
                if m.special_workspace.id != 0 {
                    active_workspaces.insert(m.special_workspace.id);
                }
            }
        }

        let mut windows = Vec::new();
        if !cli.no_window {
            let clients = Clients::get()?;
            for c in clients {
                if c.size.0 > 0
                    && c.size.1 > 0
                    && (active_workspaces.is_empty() || active_workspaces.contains(&c.workspace.id))
                {
                    windows.push(Rect {
                        x: c.at.0 as f32,
                        y: c.at.1 as f32,
                        w: c.size.0 as f32,
                        h: c.size.1 as f32,
                    });
                }
            }
        }

        let selection = Arc::new(Mutex::new(SelectionState {
            windows,
            rounding,
            current_hover: None,
            drag_start: None,
            drag_current: None,
            selected_rect: None,
            finished: false,
            screen_on_empty: cli.screen_on_empty,
            last_pos: None,
        }));

        let (min_x, min_y) = run_wayland_overlay(
            screen_w,
            screen_h,
            selection.clone(),
            screenshot.clone(),
            colors,
        )?;

        let result = selection.lock().unwrap().selected_rect;
        let (mut tr, is_w) = match result {
            Some(val) => val,
            None => {
                return Ok(());
            }
        };

        tr.x -= min_x as f32;
        tr.y -= min_y as f32;
        (tr, is_w)
    };

    let final_image = crop_and_round(
        &screenshot,
        target_rect,
        if is_window { rounding } else { 0.0 },
    );
    let mut png_bytes = Vec::new();
    final_image.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)?;

    let mut saved_path = None;
    if let Some(ref mode) = resolved_mode {
        let path = save_screenshot(mode, &png_bytes)?;
        saved_path = Some(path);
    }

    if plan.clipboard {
        copy_to_clipboard(&png_bytes)?;
    }

    if plan.stdout {
        write_png_to_stdout(&png_bytes)?;
    }

    if !plan.stdout {
        match (&saved_path, plan.clipboard) {
            (Some(path), true) => {
                println!(
                    "Screenshot saved to {} and copied to clipboard.",
                    path.display()
                );
                send_notification(
                    "Screenshot saved and copied",
                    &format!("Saved to {}", path.display()),
                    Some(path),
                );
            }
            (Some(path), false) => {
                println!("Screenshot saved to {}.", path.display());
                send_notification(
                    "Screenshot saved",
                    &format!("Saved to {}", path.display()),
                    Some(path),
                );
            }
            (None, true) => {
                println!("Screenshot copied to clipboard.");
                send_notification("Screenshot copied", "Screenshot copied to clipboard.", None);
            }
            (None, false) => {}
        }
    }

    Ok(())
}
