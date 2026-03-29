use crate::{Config, input};
use crossbeam_channel::{Receiver, unbounded};
use eframe::egui::{self, Color32, FontId, Frame, Pos2, Rect, Rounding, Stroke, Vec2};
use std::thread;
use std::time::{Duration, Instant};
use x11rb::wrapper::ConnectionExt;

fn x11_screen_size() -> Option<Vec2> {
    use x11rb::connection::Connection;
    use x11rb::rust_connection::RustConnection;
    let (conn, screen_num) = RustConnection::connect(None).ok()?;
    let s = &conn.setup().roots[screen_num];
    Some(Vec2::new(
        s.width_in_pixels as f32,
        s.height_in_pixels as f32,
    ))
}

fn apply_x11_hints(timeout: Duration) {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::*;
    use x11rb::rust_connection::RustConnection;

    let Ok((conn, screen_num)) = RustConnection::connect(None) else {
        return;
    };
    let root = conn.setup().roots[screen_num].root;

    let intern = |name: &[u8]| -> Option<u32> {
        conn.intern_atom(false, name)
            .ok()?
            .reply()
            .ok()
            .map(|r| r.atom)
    };
    let get_prop32 = |win: u32, prop: u32, ty: AtomEnum| -> Vec<u32> {
        conn.get_property(false, win, prop, ty, 0, 1024)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().map(|i| i.collect()))
            .unwrap_or_default()
    };

    let Some(net_client_list) = intern(b"_NET_CLIENT_LIST") else {
        return;
    };
    let Some(net_wm_pid) = intern(b"_NET_WM_PID") else {
        return;
    };
    let Some(net_wm_state) = intern(b"_NET_WM_STATE") else {
        return;
    };
    let Some(state_above) = intern(b"_NET_WM_STATE_ABOVE") else {
        return;
    };
    let wm_window_type = intern(b"_NET_WM_WINDOW_TYPE");
    let wm_type_dock = intern(b"_NET_WM_WINDOW_TYPE_DOCK");
    let state_skip_taskbar = intern(b"_NET_WM_STATE_SKIP_TASKBAR");
    let state_skip_pager = intern(b"_NET_WM_STATE_SKIP_PAGER");

    let our_pid = std::process::id();
    let deadline = Instant::now() + timeout;

    loop {
        for win in get_prop32(root, net_client_list, AtomEnum::WINDOW) {
            if get_prop32(win, net_wm_pid, AtomEnum::CARDINAL)
                .first()
                .copied()
                .unwrap_or(0)
                != our_pid
            {
                continue;
            }
            // Set window type to dock so it never takes focus
            if let (Some(wt), Some(wtd)) = (wm_window_type, wm_type_dock) {
                let _ = conn.change_property32(
                    PropMode::REPLACE,
                    win,
                    wt,
                    AtomEnum::ATOM,
                    &[wtd],
                );
            }
            // Set always-on-top
            let ev = ClientMessageEvent::new(32, win, net_wm_state, [1u32, state_above, 0, 1, 0]);
            let _ = conn.send_event(
                false,
                root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                ev,
            );
            // Skip taskbar and pager
            if let Some(st) = state_skip_taskbar {
                let ev = ClientMessageEvent::new(32, win, net_wm_state, [1u32, st, 0, 1, 0]);
                let _ = conn.send_event(
                    false,
                    root,
                    EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                    ev,
                );
            }
            if let Some(sp) = state_skip_pager {
                let ev = ClientMessageEvent::new(32, win, net_wm_state, [1u32, sp, 0, 1, 0]);
                let _ = conn.send_event(
                    false,
                    root,
                    EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                    ev,
                );
            }
            let _ = conn.flush();
            return;
        }
        if Instant::now() >= deadline {
            return;
        }
        thread::sleep(Duration::from_millis(200));
    }
}

const BAR_BG: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 210);
const BAR_BORDER: Color32 = Color32::from_rgba_premultiplied(26, 26, 26, 26);
const KEY_ACTIVE_BG: Color32 = Color32::from_rgba_premultiplied(36, 36, 36, 36);
const KEY_ACTIVE_BORDER: Color32 = Color32::from_rgba_premultiplied(56, 56, 56, 56);
const KEY_ACTIVE_TEXT: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 255);
const KEY_RECENT_BG: Color32 = Color32::from_rgba_premultiplied(18, 18, 18, 18);
const KEY_RECENT_BORDER: Color32 = Color32::from_rgba_premultiplied(30, 30, 30, 30);
const KEY_RECENT_TEXT: Color32 = Color32::from_rgba_premultiplied(165, 165, 165, 165);
const KEY_GHOST_TEXT: Color32 = Color32::from_rgba_premultiplied(46, 46, 46, 46);
const MOD_COLOR: Color32 = Color32::from_rgba_premultiplied(123, 159, 255, 255);
const TIMER_BG: Color32 = Color32::from_rgba_premultiplied(20, 20, 20, 20);
const TIMER_FG: Color32 = Color32::from_rgba_premultiplied(90, 90, 90, 90);

const KEY_PAD_X: f32 = 16.0;
const KEY_PAD_Y: f32 = 10.0;
const SEP_W: f32 = 1.0;
const SEP_H: f32 = 18.0;
const SEP_GAP: f32 = 8.0;
const BAR_PAD_X: f32 = 14.0;
const BAR_PAD_Y: f32 = 10.0;
const ROUNDING: f32 = 8.0;
const ARROW_W: f32 = 8.0;
const ARROW_H: f32 = 5.0;
const TIMER_H: f32 = 2.0;
const MARKER_GAP: f32 = 4.0;
const BOTTOM_MARGIN: f32 = 40.0;
const RIGHT_MARGIN: f32 = 40.0;
const LINK_GAP: f32 = 8.0;
const PROJECT_URL: &str = "https://github.com/OmChillure/keypop";
const PROJECT_LINK_LABEL: &str = "GitHub: OmChillure/KeyPop";

pub struct KeyPopApp {
    rx: Receiver<String>,
    args: Config,
    keys: Vec<String>,
    last_press: Option<Instant>,
    alpha: f32,
    screen: Vec2,
}

impl KeyPopApp {
    fn new(args: Config, rx: Receiver<String>, screen: Vec2) -> Self {
        let cap = args.keys as usize;
        Self {
            rx,
            args,
            keys: vec![String::new(); cap],
            last_press: None,
            alpha: 0.0,
            screen,
        }
    }
}

// Splits "Ctrl+Shift+A" into ("Ctrl+Shift+", "A")
fn split_mods(label: &str) -> (&str, &str) {
    if let Some(pos) = label.rfind('+') {
        (&label[..=pos], &label[pos + 1..])
    } else {
        ("", label)
    }
}

fn text_width(ui: &egui::Ui, text: &str, font: &FontId) -> f32 {
    ui.fonts(|f| {
        f.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE)
            .rect
            .width()
    })
}

fn apply_alpha(c: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_premultiplied(
        (c.r() as f32 * alpha) as u8,
        (c.g() as f32 * alpha) as u8,
        (c.b() as f32 * alpha) as u8,
        (c.a() as f32 * alpha).round() as u8,
    )
}

impl eframe::App for KeyPopApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::viewport::WindowLevel::AlwaysOnTop,
        ));

        let mut got_new = false;
        while let Ok(key) = self.rx.try_recv() {
            self.keys.remove(0);
            self.keys.push(key);
            got_new = true;
        }

        if got_new {
            self.last_press = Some(Instant::now());
            self.alpha = 1.0;
        }

        let display_dur = Duration::from_secs_f32(self.args.display_time);
        let timer_fraction = if let Some(t) = self.last_press {
            let elapsed = t.elapsed();
            if elapsed >= display_dur {
                self.alpha = 0.0;
                0.0
            } else {
                let frac = 1.0 - elapsed.as_secs_f32() / display_dur.as_secs_f32();
                self.alpha = if frac < 0.2 { frac / 0.2 } else { 1.0 };
                frac
            }
        } else {
            0.0
        };

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if self.alpha > 0.0 {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        let screen = self.screen;
        let font = FontId::monospace(self.args.font_size);

        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                if self.alpha <= 0.0 {
                    return;
                }
                let alpha = self.alpha;
                let cap = self.keys.len();

                let key_sizes: Vec<Vec2> = self
                    .keys
                    .iter()
                    .map(|k| {
                        let w = if k.is_empty() {
                            0.0
                        } else {
                            let (m, b) = split_mods(k);
                            let mw = if m.is_empty() {
                                0.0
                            } else {
                                text_width(ui, m, &font)
                            };
                            mw + text_width(ui, b, &font) + KEY_PAD_X * 2.0
                        };
                        Vec2::new(w, font.size + KEY_PAD_Y * 2.0)
                    })
                    .collect();

                let key_h = font.size + KEY_PAD_Y * 2.0;
                let keys_w: f32 = key_sizes.iter().map(|s| s.x).sum();
                let seps_w: f32 = (cap - 1) as f32 * (SEP_W + SEP_GAP * 2.0);
                let bar_w = keys_w + seps_w + BAR_PAD_X * 2.0;
                let bar_h = key_h + BAR_PAD_Y * 2.0;

                let bar_x = screen.x - bar_w - RIGHT_MARGIN;
                let bar_y = screen.y
                    - BOTTOM_MARGIN
                    - bar_h
                    - (MARKER_GAP + ARROW_H + MARKER_GAP + TIMER_H);
                let bar_rect =
                    Rect::from_min_size(Pos2::new(bar_x, bar_y), Vec2::new(bar_w, bar_h));

                let link_font = FontId::monospace((self.args.font_size * 0.42).max(10.0));
                let link_w = text_width(ui, PROJECT_LINK_LABEL, &link_font);
                let link_h = link_font.size + 2.0;
                let link_rect = Rect::from_min_size(
                    Pos2::new(
                        screen.x - RIGHT_MARGIN - link_w,
                        bar_rect.top() - link_h - LINK_GAP,
                    ),
                    Vec2::new(link_w, link_h),
                );
                let link_color = apply_alpha(Color32::WHITE, alpha);
                let link_label = egui::RichText::new(PROJECT_LINK_LABEL)
                    .font(link_font.clone())
                    .color(link_color);
                let _ = ui.put(
                    link_rect,
                    egui::Hyperlink::from_label_and_url(link_label, PROJECT_URL),
                );

                let painter = ui.painter();
                painter.rect(
                    bar_rect,
                    Rounding::same(ROUNDING),
                    apply_alpha(BAR_BG, alpha),
                    Stroke::new(0.5, apply_alpha(BAR_BORDER, alpha)),
                );

                let mut cursor_x = bar_rect.left() + BAR_PAD_X;
                let mut active_center_x = bar_rect.center().x;

                for (i, (key, size)) in self.keys.iter().zip(key_sizes.iter()).enumerate() {
                    if i > 0 {
                        cursor_x += SEP_GAP;
                        let sep_x = cursor_x + SEP_W * 0.5;
                        painter.line_segment(
                            [
                                Pos2::new(sep_x, bar_rect.center().y - SEP_H * 0.5),
                                Pos2::new(sep_x, bar_rect.center().y + SEP_H * 0.5),
                            ],
                            Stroke::new(
                                SEP_W,
                                apply_alpha(
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                                    alpha,
                                ),
                            ),
                        );
                        cursor_x += SEP_W + SEP_GAP;
                    }

                    let is_newest = i == cap - 1;
                    let is_recent = i == cap - 2;

                    let (bg, border, text_color) = if key.is_empty() {
                        (
                            Color32::TRANSPARENT,
                            Color32::TRANSPARENT,
                            Color32::TRANSPARENT,
                        )
                    } else if is_newest {
                        (KEY_ACTIVE_BG, KEY_ACTIVE_BORDER, KEY_ACTIVE_TEXT)
                    } else if is_recent {
                        (KEY_RECENT_BG, KEY_RECENT_BORDER, KEY_RECENT_TEXT)
                    } else {
                        (Color32::TRANSPARENT, Color32::TRANSPARENT, KEY_GHOST_TEXT)
                    };

                    let key_rect =
                        Rect::from_min_size(Pos2::new(cursor_x, bar_rect.top() + BAR_PAD_Y), *size);

                    if bg != Color32::TRANSPARENT {
                        painter.rect(
                            key_rect,
                            Rounding::same(ROUNDING),
                            apply_alpha(bg, alpha),
                            Stroke::new(0.5, apply_alpha(border, alpha)),
                        );
                    }

                    if !key.is_empty() {
                        let (mod_str, base_str) = split_mods(key);
                        let mod_w = if mod_str.is_empty() {
                            0.0
                        } else {
                            text_width(ui, mod_str, &font)
                        };
                        let text_y = key_rect.top() + KEY_PAD_Y;

                        if !mod_str.is_empty() {
                            painter.text(
                                Pos2::new(key_rect.left() + KEY_PAD_X, text_y),
                                egui::Align2::LEFT_TOP,
                                mod_str,
                                font.clone(),
                                apply_alpha(MOD_COLOR, alpha),
                            );
                        }
                        painter.text(
                            Pos2::new(key_rect.left() + KEY_PAD_X + mod_w, text_y),
                            egui::Align2::LEFT_TOP,
                            base_str,
                            font.clone(),
                            apply_alpha(text_color, alpha),
                        );
                    }

                    if is_newest && !key.is_empty() {
                        active_center_x = cursor_x + size.x * 0.5;
                    }

                    cursor_x += size.x;
                }

                let ax = active_center_x;
                let ay = bar_rect.bottom() + MARKER_GAP;
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(ax, ay),
                        Pos2::new(ax - ARROW_W * 0.5, ay + ARROW_H),
                        Pos2::new(ax + ARROW_W * 0.5, ay + ARROW_H),
                    ],
                    apply_alpha(Color32::WHITE, alpha),
                    Stroke::NONE,
                ));

                let timer_y = ay + ARROW_H + MARKER_GAP;
                let timer_rect = Rect::from_min_size(
                    Pos2::new(bar_rect.left(), timer_y),
                    Vec2::new(bar_rect.width(), TIMER_H),
                );
                painter.rect_filled(
                    timer_rect,
                    Rounding::same(1.0),
                    apply_alpha(TIMER_BG, alpha),
                );
                let filled_rect = Rect::from_min_size(
                    timer_rect.min,
                    Vec2::new(bar_rect.width() * timer_fraction, TIMER_H),
                );
                painter.rect_filled(
                    filled_rect,
                    Rounding::same(1.0),
                    apply_alpha(TIMER_FG, alpha),
                );
            });
    }
}

pub fn run(args: Config) {
    if std::env::var("WAYLAND_DISPLAY").is_ok() && std::env::var("DISPLAY").is_ok() {
        #[allow(deprecated)]
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY")
        };
    }

    let screen = x11_screen_size().unwrap_or(Vec2::new(1920.0, 1080.0));

    thread::spawn(|| apply_x11_hints(Duration::from_secs(3)));

    let (tx, rx) = unbounded::<String>();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_inner_size(screen)
            .with_position(egui::Pos2::ZERO)
            .with_taskbar(false),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "keypop",
        options,
        Box::new(move |cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::TRANSPARENT;
            visuals.window_fill = egui::Color32::TRANSPARENT;
            cc.egui_ctx.set_visuals(visuals);

            let egui_ctx = cc.egui_ctx.clone();
            thread::Builder::new()
                .name("keypop-input".into())
                .spawn(move || {
                    if let Err(e) = input::run(tx, egui_ctx) {
                        eprintln!("[keypop] input error: {e}");
                        eprintln!("[keypop] hint: sudo usermod -aG input $USER  (then re-login)");
                    }
                })
                .expect("failed to spawn input thread");

            Ok(Box::new(KeyPopApp::new(args, rx, screen)))
        }),
    )
    .expect("eframe failed to start");
}
