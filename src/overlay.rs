use image::RgbaImage;
use std::sync::{Arc, Mutex};
use tiny_skia::*;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
        },
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use wayland_client::{
    globals::registry_queue_init, protocol::wl_keyboard, protocol::wl_output, protocol::wl_pointer,
    protocol::wl_seat, protocol::wl_shm, protocol::wl_surface, Connection, Proxy, QueueHandle,
};
use wayland_cursor::CursorTheme;

#[derive(Clone, Copy, Debug)]
pub struct ParsedColors {
    pub stroke: [u8; 4],
    pub dim: [u8; 4],
    pub inside: [u8; 4],
}

pub fn parse_hex_color(hex: &str) -> [u8; 4] {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
        [r, g, b, 255]
    } else if hex.len() == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
        let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
        [r, g, b, a]
    } else {
        [255, 255, 255, 255]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
    pub fn area(&self) -> f32 {
        self.w * self.h
    }
}

pub struct SurfaceData {
    pub layer_surface: LayerSurface,
    pub pool: SlotPool,
    pub offset: (i32, i32),
    pub width: u32,
    pub height: u32,
    pub dimmed_bg: Vec<u8>,
    pub bright_bg: Vec<u8>,
}

pub struct SelectionState {
    pub windows: Vec<Rect>,
    pub rounding: f32,
    pub current_hover: Option<(Rect, bool)>,
    pub drag_start: Option<(f32, f32)>,
    pub drag_current: Option<(f32, f32)>,
    pub selected_rect: Option<(Rect, bool)>,
    pub finished: bool,
    pub screen_on_empty: bool,
    pub last_pos: Option<(f32, f32)>,
}

pub struct App {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm: Shm,
    pub surfaces: Vec<SurfaceData>,
    pub selection: Arc<Mutex<SelectionState>>,
    pub screenshot: Arc<RgbaImage>,
    pub cursor_theme: Option<CursorTheme>,
    pub cursor_surface: Option<wl_surface::WlSurface>,
    pub min_x: i32,
    pub min_y: i32,
    pub colors: ParsedColors,
}

pub fn run_wayland_overlay(
    screenshot_w: u32,
    screenshot_h: u32,
    selection: Arc<Mutex<SelectionState>>,
    screenshot: Arc<RgbaImage>,
    colors: ParsedColors,
) -> Result<(i32, i32), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let shm = Shm::bind(&globals, &qh)?;

    let cursor_theme = CursorTheme::load(&conn, shm.wl_shm().clone(), 24).ok();
    let cursor_surface = Some(compositor_state.create_surface(&qh));

    let mut app = App {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor_state,
        shm,
        surfaces: Vec::new(),
        selection,
        screenshot,
        cursor_theme,
        cursor_surface,
        min_x: 0,
        min_y: 0,
        colors,
    };

    event_queue.roundtrip(&mut app)?;
    event_queue.roundtrip(&mut app)?;

    let outputs: Vec<wl_output::WlOutput> = app.output_state.outputs().collect();

    if outputs.is_empty() {
        push_fallback_surface(&mut app, &layer_shell, &qh, screenshot_w, screenshot_h)?;
    } else {
        for output in &outputs {
            let info = match app.output_state.info(output) {
                Some(i) => i,
                None => continue,
            };
            let offset = info.location;
            let (w, h) = if let Some((lw, lh)) = info.logical_size {
                if lw > 0 && lh > 0 {
                    (lw as u32, lh as u32)
                } else {
                    continue;
                }
            } else if let Some(mode) = info.modes.iter().find(|m| m.current) {
                let scale = info.scale_factor.max(1) as u32;
                (
                    mode.dimensions.0 as u32 / scale,
                    mode.dimensions.1 as u32 / scale,
                )
            } else {
                continue;
            };

            let surface = app.compositor_state.create_surface(&qh);
            let ls = layer_shell.create_layer_surface(
                &qh,
                surface,
                Layer::Overlay,
                Some("hyprsnip"),
                Some(output),
            );
            ls.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
            ls.set_exclusive_zone(-1);
            ls.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
            ls.set_size(0, 0);
            ls.commit();

            let pool = SlotPool::new((w * h * 4 * 3) as usize, &app.shm)?;
            app.surfaces.push(SurfaceData {
                layer_surface: ls,
                pool,
                offset,
                width: w,
                height: h,
                dimmed_bg: Vec::new(),
                bright_bg: Vec::new(),
            });
        }

        if app.surfaces.is_empty() {
            push_fallback_surface(&mut app, &layer_shell, &qh, screenshot_w, screenshot_h)?;
        }
    }

    app.min_x = app.surfaces.iter().map(|s| s.offset.0).min().unwrap_or(0);
    app.min_y = app.surfaces.iter().map(|s| s.offset.1).min().unwrap_or(0);

    for i in 0..app.surfaces.len() {
        app.update_surface_buffers(i);
    }

    loop {
        event_queue.blocking_dispatch(&mut app)?;
        if app.selection.lock().unwrap().finished {
            break;
        }
    }
    Ok((app.min_x, app.min_y))
}

fn push_fallback_surface(
    app: &mut App,
    layer_shell: &LayerShell,
    qh: &QueueHandle<App>,
    w: u32,
    h: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let surface = app.compositor_state.create_surface(qh);
    let ls = layer_shell.create_layer_surface(qh, surface, Layer::Overlay, Some("hyprsnip"), None);
    ls.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    ls.set_exclusive_zone(-1);
    ls.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    ls.set_size(0, 0);
    ls.commit();

    let pool = SlotPool::new((w * h * 4 * 3) as usize, &app.shm)?;
    app.surfaces.push(SurfaceData {
        layer_surface: ls,
        pool,
        offset: (0, 0),
        width: w,
        height: h,
        dimmed_bg: Vec::new(),
        bright_bg: Vec::new(),
    });
    Ok(())
}

impl App {
    fn update_surface_buffers(&mut self, idx: usize) {
        let sd = &mut self.surfaces[idx];
        let (w, h) = (sd.width, sd.height);
        let (ox, oy) = sd.offset;
        let (mx, my) = (self.min_x, self.min_y);
        let total = (w * h * 4) as usize;
        let mut bright = vec![0u8; total];
        let mut dimmed = vec![0u8; total];

        let dim_color = self.colors.dim;
        let dim_alpha = dim_color[3] as u16;
        let inv_dim_alpha = 255 - dim_alpha;

        for row in 0..h {
            let gy = (oy + row as i32) - my;
            if gy < 0 || gy >= self.screenshot.height() as i32 {
                continue;
            }
            for col in 0..w {
                let gx = (ox + col as i32) - mx;
                if gx < 0 || gx >= self.screenshot.width() as i32 {
                    continue;
                }

                let p = self.screenshot.get_pixel(gx as u32, gy as u32);
                let idx = ((row * w + col) * 4) as usize;

                bright[idx] = p.0[2];
                bright[idx + 1] = p.0[1];
                bright[idx + 2] = p.0[0];
                bright[idx + 3] = p.0[3];

                dimmed[idx] =
                    ((p.0[2] as u16 * inv_dim_alpha + dim_color[2] as u16 * dim_alpha) / 255) as u8;
                dimmed[idx + 1] =
                    ((p.0[1] as u16 * inv_dim_alpha + dim_color[1] as u16 * dim_alpha) / 255) as u8;
                dimmed[idx + 2] =
                    ((p.0[0] as u16 * inv_dim_alpha + dim_color[0] as u16 * dim_alpha) / 255) as u8;
                dimmed[idx + 3] = p.0[3];
            }
        }
        sd.bright_bg = bright;
        sd.dimmed_bg = dimmed;
    }

    fn update_hover(&mut self, gx: f32, gy: f32) {
        let mut sel = self.selection.lock().unwrap();
        sel.last_pos = Some((gx, gy));
        if sel.drag_start.is_some() {
            sel.drag_current = Some((gx, gy));
            return;
        }

        let hovered_window = sel
            .windows
            .iter()
            .cloned()
            .filter(|w| w.contains(gx, gy))
            .min_by(|a, b| a.area().partial_cmp(&b.area()).unwrap());

        if let Some(w) = hovered_window {
            sel.current_hover = Some((w, true));
        } else if sel.screen_on_empty {
            let screen = self
                .surfaces
                .iter()
                .find(|sd| {
                    let sx = sd.offset.0 as f32;
                    let sy = sd.offset.1 as f32;
                    let sw = sd.width as f32;
                    let sh = sd.height as f32;
                    gx >= sx && gx < sx + sw && gy >= sy && gy < sy + sh
                })
                .map(|sd| Rect {
                    x: sd.offset.0 as f32,
                    y: sd.offset.1 as f32,
                    w: sd.width as f32,
                    h: sd.height as f32,
                });
            sel.current_hover = screen.map(|s| (s, false));
        } else {
            sel.current_hover = None;
        }
    }

    fn draw_surface(&mut self, idx: usize, _qh: &QueueHandle<Self>) {
        let sd = &mut self.surfaces[idx];
        if sd.dimmed_bg.is_empty() {
            return;
        }
        let (w, h) = (sd.width, sd.height);
        let (buffer, canvas) = match sd.pool.create_buffer(
            w as i32,
            h as i32,
            (w * 4) as i32,
            wl_shm::Format::Argb8888,
        ) {
            Ok(v) => v,
            Err(_) => return,
        };

        canvas.copy_from_slice(&sd.dimmed_bg);

        let sel = self.selection.lock().unwrap();
        let target = if let (Some(s), Some(c)) = (sel.drag_start, sel.drag_current) {
            let (dx, dy) = ((c.0 - s.0).abs(), (c.1 - s.1).abs());
            if dx < 8.0 && dy < 8.0 {
                sel.current_hover
            } else {
                Some((
                    Rect {
                        x: s.0.min(c.0),
                        y: s.1.min(c.1),
                        w: dx,
                        h: dy,
                    },
                    false,
                ))
            }
        } else {
            sel.current_hover
        };

        if let Some((tg, is_rnd)) = target {
            let local = Rect {
                x: tg.x - sd.offset.0 as f32,
                y: tg.y - sd.offset.1 as f32,
                w: tg.w,
                h: tg.h,
            };
            let x1 = local.x.max(0.0).floor() as usize;
            let y1 = local.y.max(0.0).floor() as usize;
            let x2 = (local.x + local.w).min(w as f32).ceil() as usize;
            let y2 = (local.y + local.h).min(h as f32).ceil() as usize;

            if x2 > x1 && y2 > y1 && !((x2 - x1) <= 4 || (y2 - y1) <= 4) {
                let radius = if is_rnd { sel.rounding } else { 0.0 };

                for row in y1..y2 {
                    let py = row as f32 + 0.5;
                    let row_off = row * w as usize * 4;
                    for col in x1..x2 {
                        let px = col as f32 + 0.5;
                        if is_inside_rounded_rect(
                            px, py, local.x, local.y, local.w, local.h, radius,
                        ) {
                            let i = row_off + col * 4;
                            canvas[i..i + 4].copy_from_slice(&sd.bright_bg[i..i + 4]);
                        }
                    }
                }

                if let Some(mut pixmap) = PixmapMut::from_bytes(canvas, w, h) {
                    if let Some(path) = build_rounded_rect_path(local, radius) {
                        if self.colors.inside[3] > 0 {
                            let mut inside_p = Paint::default();
                            inside_p.set_color_rgba8(
                                self.colors.inside[2],
                                self.colors.inside[1],
                                self.colors.inside[0],
                                self.colors.inside[3],
                            );
                            pixmap.fill_path(
                                &path,
                                &inside_p,
                                FillRule::Winding,
                                Transform::identity(),
                                None,
                            );
                        }

                        let mut stroke_p = Paint::default();
                        stroke_p.set_color_rgba8(
                            self.colors.stroke[2],
                            self.colors.stroke[1],
                            self.colors.stroke[0],
                            self.colors.stroke[3],
                        );
                        stroke_p.anti_alias = true;
                        pixmap.stroke_path(
                            &path,
                            &stroke_p,
                            &Stroke {
                                width: 2.0,
                                ..Default::default()
                            },
                            Transform::identity(),
                            None,
                        );
                    }
                }
            }
        }

        let ls = &self.surfaces[idx].layer_surface;
        ls.wl_surface().attach(Some(buffer.wl_buffer()), 0, 0);
        ls.wl_surface().damage_buffer(0, 0, w as i32, h as i32);
        ls.wl_surface().commit();
    }

    fn draw_all(&mut self, qh: &QueueHandle<Self>) {
        for i in 0..self.surfaces.len() {
            self.draw_surface(i, qh);
        }
    }

    fn surface_offset(&self, s: &wl_surface::WlSurface) -> (i32, i32) {
        self.surfaces
            .iter()
            .find(|sd| sd.layer_surface.wl_surface().id() == s.id())
            .map(|sd| sd.offset)
            .unwrap_or((0, 0))
    }

    fn set_system_cursor(&mut self, p: &wl_pointer::WlPointer, s: u32) {
        let Some(t) = self.cursor_theme.as_mut() else {
            return;
        };
        let Some(surf) = self.cursor_surface.as_ref() else {
            return;
        };

        let cursor = if t.get_cursor("crosshair").is_some() {
            t.get_cursor("crosshair")
        } else {
            t.get_cursor("left_ptr")
        };
        let Some(cursor) = cursor else { return };

        let img = &cursor[0];
        surf.attach(Some(img), 0, 0);
        surf.damage_buffer(0, 0, img.dimensions().0 as i32, img.dimensions().1 as i32);
        surf.commit();
        p.set_cursor(
            s,
            Some(surf),
            img.hotspot().0 as i32,
            img.hotspot().1 as i32,
        );
    }
}

fn is_inside_rounded_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> bool {
    if px < x || px > x + w || py < y || py > y + h {
        return false;
    }
    let cx = if px < x + r {
        x + r
    } else if px > x + w - r {
        x + w - r
    } else {
        return true;
    };
    let cy = if py < y + r {
        y + r
    } else if py > y + h - r {
        y + h - r
    } else {
        return true;
    };
    let (dx, dy) = (px - cx, py - cy);
    dx * dx + dy * dy <= r * r
}

fn build_rounded_rect_path(rect: Rect, r: f32) -> Option<Path> {
    let (x, y, w, h) = (rect.x, rect.y, rect.w, rect.h);
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    if r <= 0.0 {
        let skia_rect = tiny_skia::Rect::from_xywh(x, y, w, h)?;
        pb.push_rect(skia_rect);
    } else {
        pb.move_to(x + r, y);
        pb.line_to(x + w - r, y);
        pb.quad_to(x + w, y, x + w, y + r);
        pb.line_to(x + w, y + h - r);
        pb.quad_to(x + w, y + h, x + w - r, y + h);
        pb.line_to(x + r, y + h);
        pb.quad_to(x, y + h, x, y + h - r);
        pb.line_to(x, y + r);
        pb.quad_to(x, y, x + r, y);
        pb.close();
    }
    pb.finish()
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.selection.lock().unwrap().finished = true;
    }
    fn configure(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        ls: &LayerSurface,
        c: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _: u32,
    ) {
        if let Some(i) = self
            .surfaces
            .iter()
            .position(|s| s.layer_surface.wl_surface().id() == ls.wl_surface().id())
        {
            let new_w = if c.new_size.0 > 0 {
                c.new_size.0
            } else {
                self.surfaces[i].width
            };
            let new_h = if c.new_size.1 > 0 {
                c.new_size.1
            } else {
                self.surfaces[i].height
            };

            if new_w != self.surfaces[i].width
                || new_h != self.surfaces[i].height
                || self.surfaces[i].dimmed_bg.is_empty()
            {
                self.surfaces[i].width = new_w;
                self.surfaces[i].height = new_h;
                self.update_surface_buffers(i);
            }
            self.draw_surface(i, qh);
        }
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        s: wl_seat::WlSeat,
        c: Capability,
    ) {
        if c == Capability::Pointer {
            let _ = self.seat_state.get_pointer(qh, &s);
        }
        if c == Capability::Keyboard {
            let _ = self.seat_state.get_keyboard(qh, &s, None);
        }
    }
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        p: &wl_pointer::WlPointer,
        evs: &[PointerEvent],
    ) {
        let mut redraw = false;
        for ev in evs {
            let (ox, oy) = self.surface_offset(&ev.surface);
            let (gx, gy) = (
                ev.position.0 as f32 + ox as f32,
                ev.position.1 as f32 + oy as f32,
            );
            match ev.kind {
                PointerEventKind::Enter { serial } => {
                    self.set_system_cursor(p, serial);
                    redraw = true;
                }
                PointerEventKind::Motion { .. } => {
                    self.update_hover(gx, gy);
                    redraw = true;
                }
                PointerEventKind::Press { button: 0x110, .. } => {
                    let mut sel = self.selection.lock().unwrap();
                    sel.drag_start = Some((gx, gy));
                    sel.drag_current = Some((gx, gy));
                    redraw = true;
                }
                PointerEventKind::Release { button: 0x110, .. } => {
                    let mut sel = self.selection.lock().unwrap();
                    if let (Some(s), Some(c)) = (sel.drag_start, sel.drag_current) {
                        let (dx, dy) = ((c.0 - s.0).abs(), (c.1 - s.1).abs());
                        if dx < 8.0 && dy < 8.0 {
                            if let Some(h) = sel.current_hover {
                                sel.selected_rect = Some(h);
                                sel.finished = true;
                                return;
                            } else {
                                sel.drag_start = None;
                                sel.drag_current = None;
                            }
                        } else {
                            sel.selected_rect = Some((
                                Rect {
                                    x: s.0.min(c.0),
                                    y: s.1.min(c.1),
                                    w: dx,
                                    h: dy,
                                },
                                false,
                            ));
                            sel.finished = true;
                            return;
                        }
                    }
                    redraw = true;
                }
                _ => {}
            }
        }
        if redraw {
            self.draw_all(qh);
        }
    }
}

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
    }
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
    }
    fn press_key(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        e: KeyEvent,
    ) {
        if e.raw_code == 1 {
            self.selection.lock().unwrap().finished = true;
        } else if e.raw_code == 57 {
            let last_pos = {
                let mut sel = self.selection.lock().unwrap();
                sel.screen_on_empty = !sel.screen_on_empty;
                sel.last_pos
            };
            if let Some((gx, gy)) = last_pos {
                self.update_hover(gx, gy);
            }
            self.draw_all(qh);
        }
    }
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }
    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: Modifiers,
        _: u32,
    ) {
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(App);
delegate_output!(App);
delegate_shm!(App);
delegate_seat!(App);
delegate_pointer!(App);
delegate_keyboard!(App);
delegate_layer!(App);
delegate_registry!(App);

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers!(OutputState, SeatState);
}

pub fn crop_and_round(src: &RgbaImage, rect: Rect, rounding: f32) -> RgbaImage {
    let (rx, ry) = (rect.x.max(0.0) as u32, rect.y.max(0.0) as u32);
    let rw = (rect.w as u32).min(src.width().saturating_sub(rx));
    let rh = (rect.h as u32).min(src.height().saturating_sub(ry));
    let mut out = image::ImageBuffer::new(rw, rh);
    for y in 0..rh {
        for x in 0..rw {
            let p = *src.get_pixel(rx + x, ry + y);
            if rounding > 0.0 {
                let (cx, cy) = (
                    if x < rounding as u32 {
                        rounding - x as f32
                    } else if x >= rw - rounding as u32 {
                        x as f32 - (rw as f32 - rounding)
                    } else {
                        0.0
                    },
                    if y < rounding as u32 {
                        rounding - y as f32
                    } else if y >= rh - rounding as u32 {
                        y as f32 - (rh as f32 - rounding)
                    } else {
                        0.0
                    },
                );
                if cx > 0.0 && cy > 0.0 && (cx * cx + cy * cy).sqrt() > rounding {
                    out.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
                    continue;
                }
            }
            out.put_pixel(x, y, p);
        }
    }
    out
}
