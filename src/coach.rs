use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Rounding, Stroke, Vec2};

// ---------------------------------------------------------------------------
// Visual constants for the coach panel
// ---------------------------------------------------------------------------

const PANEL_BG: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 210);
const PANEL_BORDER: Color32 = Color32::from_rgba_premultiplied(40, 40, 40, 40);
const PANEL_ROUNDING: f32 = 8.0;
const PANEL_PAD: f32 = 12.0;
const SCORECARD_H: f32 = 28.0;
const HEATMAP_GAP: f32 = 10.0;
const KEY_UNIT: f32 = 28.0;
const KEY_GAP: f32 = 2.0;
const KEY_ROUNDING: f32 = 3.0;
const CPM_WINDOW_SECS: f32 = 10.0;

// ---------------------------------------------------------------------------
// Coach configuration (from CLI flags)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CoachConfig {
    pub target_cpm: u32,
    pub show_heatmap: bool,
    pub show_fingers: bool,
}

impl Default for CoachConfig {
    fn default() -> Self {
        Self {
            target_cpm: 200,
            show_heatmap: true,
            show_fingers: true,
        }
    }
}

// ---------------------------------------------------------------------------
// QWERTY keyboard layout
// ---------------------------------------------------------------------------

pub struct KeyDef {
    pub label: &'static str,
    pub width: f32,
    pub finger: u8,
}

pub fn qwerty_layout() -> Vec<Vec<KeyDef>> {
    vec![
        // Row 0: number row
        vec![
            KeyDef { label: "`",    width: 1.0,  finger: 0 },
            KeyDef { label: "1",    width: 1.0,  finger: 0 },
            KeyDef { label: "2",    width: 1.0,  finger: 1 },
            KeyDef { label: "3",    width: 1.0,  finger: 2 },
            KeyDef { label: "4",    width: 1.0,  finger: 3 },
            KeyDef { label: "5",    width: 1.0,  finger: 3 },
            KeyDef { label: "6",    width: 1.0,  finger: 4 },
            KeyDef { label: "7",    width: 1.0,  finger: 4 },
            KeyDef { label: "8",    width: 1.0,  finger: 5 },
            KeyDef { label: "9",    width: 1.0,  finger: 6 },
            KeyDef { label: "0",    width: 1.0,  finger: 7 },
            KeyDef { label: "-",    width: 1.0,  finger: 7 },
            KeyDef { label: "=",    width: 1.0,  finger: 7 },
            KeyDef { label: "Bksp", width: 2.0,  finger: 7 },
        ],
        // Row 1: QWERTY
        vec![
            KeyDef { label: "Tab",  width: 1.5,  finger: 0 },
            KeyDef { label: "Q",    width: 1.0,  finger: 0 },
            KeyDef { label: "W",    width: 1.0,  finger: 1 },
            KeyDef { label: "E",    width: 1.0,  finger: 2 },
            KeyDef { label: "R",    width: 1.0,  finger: 3 },
            KeyDef { label: "T",    width: 1.0,  finger: 3 },
            KeyDef { label: "Y",    width: 1.0,  finger: 4 },
            KeyDef { label: "U",    width: 1.0,  finger: 4 },
            KeyDef { label: "I",    width: 1.0,  finger: 5 },
            KeyDef { label: "O",    width: 1.0,  finger: 6 },
            KeyDef { label: "P",    width: 1.0,  finger: 7 },
            KeyDef { label: "[",    width: 1.0,  finger: 7 },
            KeyDef { label: "]",    width: 1.0,  finger: 7 },
            KeyDef { label: "\\",   width: 1.5,  finger: 7 },
        ],
        // Row 2: home row
        vec![
            KeyDef { label: "Caps",  width: 1.75, finger: 0 },
            KeyDef { label: "A",     width: 1.0,  finger: 0 },
            KeyDef { label: "S",     width: 1.0,  finger: 1 },
            KeyDef { label: "D",     width: 1.0,  finger: 2 },
            KeyDef { label: "F",     width: 1.0,  finger: 3 },
            KeyDef { label: "G",     width: 1.0,  finger: 3 },
            KeyDef { label: "H",     width: 1.0,  finger: 4 },
            KeyDef { label: "J",     width: 1.0,  finger: 4 },
            KeyDef { label: "K",     width: 1.0,  finger: 5 },
            KeyDef { label: "L",     width: 1.0,  finger: 6 },
            KeyDef { label: ";",     width: 1.0,  finger: 7 },
            KeyDef { label: "'",     width: 1.0,  finger: 7 },
            KeyDef { label: "Enter", width: 2.25, finger: 7 },
        ],
        // Row 3: shift row
        vec![
            KeyDef { label: "Shift", width: 2.25, finger: 0 },
            KeyDef { label: "Z",     width: 1.0,  finger: 0 },
            KeyDef { label: "X",     width: 1.0,  finger: 1 },
            KeyDef { label: "C",     width: 1.0,  finger: 2 },
            KeyDef { label: "V",     width: 1.0,  finger: 3 },
            KeyDef { label: "B",     width: 1.0,  finger: 3 },
            KeyDef { label: "N",     width: 1.0,  finger: 4 },
            KeyDef { label: "M",     width: 1.0,  finger: 4 },
            KeyDef { label: ",",     width: 1.0,  finger: 5 },
            KeyDef { label: ".",     width: 1.0,  finger: 6 },
            KeyDef { label: "/",     width: 1.0,  finger: 7 },
            KeyDef { label: "Shift", width: 2.75, finger: 7 },
        ],
        // Row 4: bottom row
        vec![
            KeyDef { label: "Ctrl",  width: 1.25, finger: 0 },
            KeyDef { label: "Super", width: 1.25, finger: 8 },
            KeyDef { label: "Alt",   width: 1.25, finger: 8 },
            KeyDef { label: "Space", width: 6.25, finger: 8 },
            KeyDef { label: "AltGr", width: 1.25, finger: 8 },
            KeyDef { label: "Super", width: 1.25, finger: 8 },
            KeyDef { label: "Menu",  width: 1.25, finger: 8 },
            KeyDef { label: "Ctrl",  width: 1.25, finger: 7 },
        ],
    ]
}

pub fn keyboard_width(layout: &[Vec<KeyDef>]) -> f32 {
    layout
        .iter()
        .map(|row| {
            let keys_w: f32 = row.iter().map(|k| k.width * KEY_UNIT).sum();
            let gaps_w = (row.len().saturating_sub(1)) as f32 * KEY_GAP;
            keys_w + gaps_w
        })
        .fold(0.0f32, f32::max)
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    (
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t) as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t) as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t) as u8,
    )
}

pub fn heat_color(count: u32, max: u32) -> Color32 {
    if max == 0 || count == 0 {
        return Color32::from_rgb(40, 40, 50);
    }
    let t = (count as f32) / (max as f32);
    let (r, g, b) = if t < 0.25 {
        lerp_rgb((40, 60, 120), (0, 160, 200), t / 0.25)
    } else if t < 0.5 {
        lerp_rgb((0, 160, 200), (0, 200, 60), (t - 0.25) / 0.25)
    } else if t < 0.75 {
        lerp_rgb((0, 200, 60), (230, 200, 0), (t - 0.5) / 0.25)
    } else {
        lerp_rgb((230, 200, 0), (230, 55, 30), (t - 0.75) / 0.25)
    };
    Color32::from_rgb(r, g, b)
}

pub fn finger_color(zone: u8) -> Color32 {
    match zone {
        0 => Color32::from_rgba_premultiplied(180, 70, 70, 55),
        1 => Color32::from_rgba_premultiplied(180, 130, 50, 55),
        2 => Color32::from_rgba_premultiplied(180, 180, 50, 55),
        3 => Color32::from_rgba_premultiplied(50, 180, 50, 55),
        4 => Color32::from_rgba_premultiplied(50, 180, 180, 55),
        5 => Color32::from_rgba_premultiplied(50, 90, 180, 55),
        6 => Color32::from_rgba_premultiplied(130, 50, 180, 55),
        7 => Color32::from_rgba_premultiplied(180, 50, 130, 55),
        8 => Color32::from_rgba_premultiplied(110, 110, 110, 55),
        _ => Color32::from_rgba_premultiplied(50, 50, 50, 55),
    }
}

// ---------------------------------------------------------------------------
// Coach state — tracks all per-session typing statistics
// ---------------------------------------------------------------------------

pub struct CoachState {
    pub config: CoachConfig,
    pub total_presses: u64,
    pub backspace_count: u64,
    pub press_times: VecDeque<Instant>,
    pub session_start: Instant,
    pub key_counts: HashMap<String, u32>,
    pub key_history: Vec<String>,
    pub max_count: u32,
    pub above_target_since: Option<Instant>,
}

impl CoachState {
    pub fn new(config: CoachConfig) -> Self {
        Self {
            config,
            total_presses: 0,
            backspace_count: 0,
            press_times: VecDeque::new(),
            session_start: Instant::now(),
            key_counts: HashMap::new(),
            key_history: Vec::new(),
            max_count: 0,
            above_target_since: None,
        }
    }

    pub fn record_press(&mut self, key: &str) {
        self.record_press_at(key, Instant::now());
    }

    pub fn record_press_at(&mut self, key: &str, now: Instant) {
        self.total_presses += 1;
        self.press_times.push_back(now);

        let base = base_key(key);

        if base == "Bksp" {
            self.backspace_count += 1;
            if let Some(last) = self.key_history.pop() {
                if let Some(count) = self.key_counts.get_mut(&last) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        self.key_counts.remove(&last);
                    }
                }
                self.recalc_max();
            }
        } else {
            let entry = self.key_counts.entry(base.to_string()).or_insert(0);
            *entry += 1;
            if *entry > self.max_count {
                self.max_count = *entry;
            }
            self.key_history.push(base.to_string());
        }
    }

    fn recalc_max(&mut self) {
        self.max_count = self.key_counts.values().copied().max().unwrap_or(0);
    }

    pub fn cpm_at(&self, now: Instant) -> f32 {
        let window = Duration::from_secs_f32(CPM_WINDOW_SECS);
        let count = self
            .press_times
            .iter()
            .filter(|&&t| now.duration_since(t) <= window)
            .count() as f32;
        let session_elapsed = now.duration_since(self.session_start).as_secs_f32();
        let effective = session_elapsed.min(CPM_WINDOW_SECS);
        if effective < 0.1 {
            return 0.0;
        }
        count / (effective / 60.0)
    }

    #[allow(dead_code)]
    pub fn cpm(&self) -> f32 {
        self.cpm_at(Instant::now())
    }

    pub fn wpm_at(&self, now: Instant) -> f32 {
        self.cpm_at(now) / 5.0
    }

    #[allow(dead_code)]
    pub fn wpm(&self) -> f32 {
        self.wpm_at(Instant::now())
    }

    pub fn accuracy(&self) -> f32 {
        if self.total_presses == 0 {
            return 100.0;
        }
        (self.total_presses - self.backspace_count) as f32 / self.total_presses as f32 * 100.0
    }

    pub fn update_streak_at(&mut self, now: Instant) {
        let cpm = self.cpm_at(now);
        if cpm >= self.config.target_cpm as f32 {
            if self.above_target_since.is_none() {
                self.above_target_since = Some(now);
            }
        } else {
            self.above_target_since = None;
        }
    }

    pub fn streak_secs_at(&self, now: Instant) -> f32 {
        match self.above_target_since {
            Some(since) => now.duration_since(since).as_secs_f32(),
            None => 0.0,
        }
    }

    #[allow(dead_code)]
    pub fn streak_secs(&self) -> f32 {
        self.streak_secs_at(Instant::now())
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    pub fn draw(&mut self, ui: &egui::Ui, bottom_y: f32, right_edge: f32, font_size: f32) {
        let now = Instant::now();
        self.update_streak_at(now);
        self.prune_old_presses(now);

        let layout = qwerty_layout();
        let kb_w = keyboard_width(&layout);
        let kb_h = if self.config.show_heatmap {
            layout.len() as f32 * (KEY_UNIT + KEY_GAP) - KEY_GAP
        } else {
            0.0
        };

        let gap = if self.config.show_heatmap {
            HEATMAP_GAP
        } else {
            0.0
        };
        let inner_h = SCORECARD_H + gap + kb_h;
        let inner_w = kb_w.max(350.0);

        let panel_w = inner_w + PANEL_PAD * 2.0;
        let panel_h = inner_h + PANEL_PAD * 2.0;

        let panel_rect = Rect::from_min_size(
            Pos2::new(right_edge - panel_w, bottom_y - panel_h),
            Vec2::new(panel_w, panel_h),
        );

        let painter = ui.painter();

        painter.rect(
            panel_rect,
            Rounding::same(PANEL_ROUNDING),
            PANEL_BG,
            Stroke::new(0.5, PANEL_BORDER),
        );

        let sc_y = panel_rect.top() + PANEL_PAD;
        let sc_x = panel_rect.left() + PANEL_PAD;
        self.draw_scorecard(ui, painter, sc_x, sc_y, inner_w, font_size, now);

        if self.config.show_heatmap {
            let hm_y = sc_y + SCORECARD_H + gap;
            let hm_x = panel_rect.left() + PANEL_PAD + (inner_w - kb_w) / 2.0;
            self.draw_heatmap(painter, &layout, hm_x, hm_y, font_size);
        }
    }

    fn draw_scorecard(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        x: f32,
        y: f32,
        w: f32,
        font_size: f32,
        now: Instant,
    ) {
        let font = FontId::monospace((font_size * 0.5).max(11.0));
        let cpm = self.cpm_at(now);
        let wpm = self.wpm_at(now);
        let acc = self.accuracy();
        let streak = self.streak_secs_at(now);
        let above = cpm >= self.config.target_cpm as f32;

        let sep_color = Color32::from_rgb(60, 60, 60);
        let dim = Color32::from_rgb(180, 180, 180);
        let bright = Color32::from_rgb(100, 255, 100);
        let streak_color = if above {
            Color32::from_rgb(255, 200, 50)
        } else {
            Color32::from_rgb(90, 90, 90)
        };

        let cpm_color = if above { bright } else { dim };

        let items: Vec<(String, Color32)> = vec![
            (format!("CPM {:.0}", cpm), cpm_color),
            (format!("WPM {:.0}", wpm), dim),
            (
                format!("ACC {:.0}%", acc),
                if acc >= 95.0 {
                    bright
                } else if acc >= 85.0 {
                    Color32::from_rgb(230, 200, 50)
                } else {
                    Color32::from_rgb(230, 80, 60)
                },
            ),
            (
                if above {
                    format!("STREAK {:.0}s", streak)
                } else {
                    "STREAK --".into()
                },
                streak_color,
            ),
        ];

        let sep_gap = 10.0;
        let mut cx = x;
        for (i, (text, color)) in items.iter().enumerate() {
            if i > 0 {
                painter.line_segment(
                    [
                        Pos2::new(cx, y + 3.0),
                        Pos2::new(cx, y + font.size - 3.0),
                    ],
                    Stroke::new(1.0, sep_color),
                );
                cx += sep_gap;
            }
            painter.text(
                Pos2::new(cx, y),
                egui::Align2::LEFT_TOP,
                text,
                font.clone(),
                *color,
            );
            cx += measure_text(ui, text, &font) + sep_gap;
        }

        // Target indicator (right-aligned)
        let target_text = format!("target {} CPM", self.config.target_cpm);
        let target_font = FontId::monospace((font_size * 0.38).max(9.0));
        let tw = measure_text(ui, &target_text, &target_font);
        painter.text(
            Pos2::new(x + w - tw, y + font.size - target_font.size),
            egui::Align2::LEFT_TOP,
            &target_text,
            target_font,
            Color32::from_rgb(80, 80, 80),
        );
    }

    fn draw_heatmap(
        &self,
        painter: &egui::Painter,
        layout: &[Vec<KeyDef>],
        x: f32,
        y: f32,
        font_size: f32,
    ) {
        let key_font = FontId::monospace((font_size * 0.35).max(8.0));

        for (ri, row) in layout.iter().enumerate() {
            let ry = y + ri as f32 * (KEY_UNIT + KEY_GAP);
            let mut kx = x;

            for kdef in row {
                let kw = kdef.width * KEY_UNIT + (kdef.width - 1.0).max(0.0) * KEY_GAP;
                let key_rect =
                    Rect::from_min_size(Pos2::new(kx, ry), Vec2::new(kw, KEY_UNIT));

                let count = self.key_counts.get(kdef.label).copied().unwrap_or(0);
                let bg = heat_color(count, self.max_count);
                painter.rect(
                    key_rect,
                    Rounding::same(KEY_ROUNDING),
                    bg,
                    Stroke::new(0.5, Color32::from_rgb(55, 55, 55)),
                );

                // finger zone bar at bottom of key
                if self.config.show_fingers {
                    let bar_h = 3.0;
                    let bar_rect = Rect::from_min_size(
                        Pos2::new(
                            key_rect.left() + 1.0,
                            key_rect.bottom() - bar_h - 1.0,
                        ),
                        Vec2::new(key_rect.width() - 2.0, bar_h),
                    );
                    painter.rect_filled(
                        bar_rect,
                        Rounding::same(1.0),
                        finger_color(kdef.finger),
                    );
                }

                // key label
                let label_color = if count > 0 {
                    Color32::from_rgba_premultiplied(255, 255, 255, 220)
                } else {
                    Color32::from_rgba_premultiplied(140, 140, 140, 140)
                };
                painter.text(
                    key_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    kdef.label,
                    key_font.clone(),
                    label_color,
                );

                kx += kw + KEY_GAP;
            }
        }
    }

    fn prune_old_presses(&mut self, now: Instant) {
        let window = Duration::from_secs_f32(CPM_WINDOW_SECS);
        while let Some(&front) = self.press_times.front() {
            if now.duration_since(front) > window {
                self.press_times.pop_front();
            } else {
                break;
            }
        }
    }
}

fn base_key(label: &str) -> &str {
    label.rsplit('+').next().unwrap_or(label)
}

fn measure_text(ui: &egui::Ui, text: &str, font: &FontId) -> f32 {
    ui.fonts(|f| {
        f.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE)
            .rect
            .width()
    })
}

// ---------------------------------------------------------------------------
// Entry point: runs the overlay with the coach panel
// ---------------------------------------------------------------------------

pub fn run(args: crate::Config, coach_config: CoachConfig) {
    crate::overlay::run_with_input(args, move |tx, egui_ctx, args, rx, screen| {
        std::thread::Builder::new()
            .name("keypop-input".into())
            .spawn(move || {
                if let Err(e) = crate::input::run(tx, egui_ctx) {
                    eprintln!("[keypop] input error: {e}");
                    eprintln!("[keypop] hint: sudo usermod -aG input $USER  (then re-login)");
                }
            })
            .expect("failed to spawn input thread");

        let coach_state = CoachState::new(coach_config);
        crate::overlay::KeyPopApp::new(args, rx, screen).with_coach(coach_state)
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(target_cpm: u32) -> CoachState {
        CoachState::new(CoachConfig {
            target_cpm,
            show_heatmap: true,
            show_fingers: true,
        })
    }

    fn make_state_at(target_cpm: u32, start: Instant) -> CoachState {
        CoachState {
            config: CoachConfig {
                target_cpm,
                show_heatmap: true,
                show_fingers: true,
            },
            total_presses: 0,
            backspace_count: 0,
            press_times: VecDeque::new(),
            session_start: start,
            key_counts: HashMap::new(),
            key_history: Vec::new(),
            max_count: 0,
            above_target_since: None,
        }
    }

    // -- record_press tests --

    #[test]
    fn test_record_press_increments_total() {
        let mut s = make_state(200);
        s.record_press("A");
        assert_eq!(s.total_presses, 1);
        s.record_press("B");
        assert_eq!(s.total_presses, 2);
    }

    #[test]
    fn test_record_press_updates_heatmap() {
        let mut s = make_state(200);
        s.record_press("A");
        s.record_press("A");
        s.record_press("B");
        assert_eq!(s.key_counts.get("A"), Some(&2));
        assert_eq!(s.key_counts.get("B"), Some(&1));
    }

    #[test]
    fn test_record_press_modifier_combo_tracks_base_key() {
        let mut s = make_state(200);
        s.record_press("Ctrl+C");
        s.record_press("Shift+A");
        assert_eq!(s.key_counts.get("C"), Some(&1));
        assert_eq!(s.key_counts.get("A"), Some(&1));
        assert!(s.key_counts.get("Ctrl+C").is_none());
    }

    #[test]
    fn test_record_press_tracks_history() {
        let mut s = make_state(200);
        s.record_press("A");
        s.record_press("B");
        s.record_press("C");
        assert_eq!(s.key_history, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_max_count_updates() {
        let mut s = make_state(200);
        s.record_press("A");
        assert_eq!(s.max_count, 1);
        s.record_press("A");
        assert_eq!(s.max_count, 2);
        s.record_press("B");
        assert_eq!(s.max_count, 2);
        s.record_press("B");
        s.record_press("B");
        assert_eq!(s.max_count, 3);
    }

    // -- Backspace tests --

    #[test]
    fn test_backspace_increments_backspace_count() {
        let mut s = make_state(200);
        s.record_press("A");
        s.record_press("Bksp");
        assert_eq!(s.backspace_count, 1);
        assert_eq!(s.total_presses, 2);
    }

    #[test]
    fn test_backspace_undoes_last_key_in_heatmap() {
        let mut s = make_state(200);
        s.record_press("A");
        s.record_press("B");
        assert_eq!(s.key_counts.get("B"), Some(&1));
        s.record_press("Bksp");
        assert!(s.key_counts.get("B").is_none());
        assert_eq!(s.key_counts.get("A"), Some(&1));
        assert_eq!(s.key_history, vec!["A"]);
    }

    #[test]
    fn test_backspace_decrements_count_not_remove_when_multiple() {
        let mut s = make_state(200);
        s.record_press("A");
        s.record_press("A");
        assert_eq!(s.key_counts.get("A"), Some(&2));
        s.record_press("Bksp");
        assert_eq!(s.key_counts.get("A"), Some(&1));
    }

    #[test]
    fn test_backspace_on_empty_history() {
        let mut s = make_state(200);
        s.record_press("Bksp");
        assert_eq!(s.backspace_count, 1);
        assert_eq!(s.total_presses, 1);
        assert!(s.key_counts.is_empty());
    }

    #[test]
    fn test_backspace_with_modifier() {
        let mut s = make_state(200);
        s.record_press("A");
        s.record_press("Ctrl+Bksp");
        assert_eq!(s.backspace_count, 1);
        assert!(s.key_counts.get("A").is_none());
    }

    #[test]
    fn test_max_count_recalculated_after_backspace() {
        let mut s = make_state(200);
        s.record_press("A");
        s.record_press("A");
        s.record_press("A");
        assert_eq!(s.max_count, 3);
        s.record_press("Bksp");
        assert_eq!(s.max_count, 2);
        s.record_press("Bksp");
        assert_eq!(s.max_count, 1);
        s.record_press("Bksp");
        assert_eq!(s.max_count, 0);
    }

    // -- Accuracy tests --

    #[test]
    fn test_accuracy_all_correct() {
        let mut s = make_state(200);
        s.record_press("A");
        s.record_press("B");
        s.record_press("C");
        assert!((s.accuracy() - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_accuracy_with_backspaces() {
        let mut s = make_state(200);
        for _ in 0..9 {
            s.record_press("A");
        }
        s.record_press("Bksp");
        // 10 total, 1 backspace → (10-1)/10 = 90%
        assert!((s.accuracy() - 90.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_accuracy_zero_presses() {
        let s = make_state(200);
        assert!((s.accuracy() - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_accuracy_half_backspaces() {
        let mut s = make_state(200);
        s.record_press("A");
        s.record_press("Bksp");
        // 2 total, 1 backspace → 50%
        assert!((s.accuracy() - 50.0).abs() < f32::EPSILON);
    }

    // -- CPM tests --

    #[test]
    fn test_cpm_at_with_known_presses() {
        let base = Instant::now();
        let mut s = make_state_at(200, base);

        // 20 presses over 10 seconds (2 per second)
        for i in 0..20 {
            let t = base + Duration::from_millis(i * 500);
            s.press_times.push_back(t);
            s.total_presses += 1;
        }

        // Query at t=10s. All 20 presses in the 10s window.
        // CPM = 20 / (10/60) = 120
        let now = base + Duration::from_secs(10);
        let cpm = s.cpm_at(now);
        assert!(
            (cpm - 120.0).abs() < 2.0,
            "expected ~120 CPM, got {cpm}"
        );
    }

    #[test]
    fn test_cpm_at_only_counts_window() {
        let base = Instant::now();
        let mut s = make_state_at(200, base);

        // 10 presses in first 5 seconds
        for i in 0..10 {
            s.press_times.push_back(base + Duration::from_millis(i * 500));
        }
        // 10 presses in seconds 15-20 (after a gap)
        for i in 0..10 {
            s.press_times
                .push_back(base + Duration::from_millis(15000 + i * 500));
        }

        // Query at t=20s. Window is [10s, 20s]. Only the second batch (10 presses) is inside.
        // CPM = 10 / (10/60) = 60
        let now = base + Duration::from_secs(20);
        let cpm = s.cpm_at(now);
        assert!(
            (cpm - 60.0).abs() < 2.0,
            "expected ~60 CPM, got {cpm}"
        );
    }

    #[test]
    fn test_cpm_at_zero_when_no_presses() {
        let base = Instant::now();
        let s = make_state_at(200, base);
        let cpm = s.cpm_at(base + Duration::from_secs(5));
        assert!((cpm - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_cpm_short_session() {
        let base = Instant::now();
        let mut s = make_state_at(200, base);
        // 10 presses in 2 seconds
        for i in 0..10 {
            s.press_times.push_back(base + Duration::from_millis(i * 200));
        }
        // At t=2s, session < window. effective = 2s. CPM = 10 / (2/60) = 300
        let now = base + Duration::from_secs(2);
        let cpm = s.cpm_at(now);
        assert!(
            (cpm - 300.0).abs() < 5.0,
            "expected ~300 CPM, got {cpm}"
        );
    }

    // -- WPM tests --

    #[test]
    fn test_wpm_is_cpm_div_5() {
        let base = Instant::now();
        let mut s = make_state_at(200, base);
        for i in 0..50 {
            s.press_times.push_back(base + Duration::from_millis(i * 200));
        }
        let now = base + Duration::from_secs(10);
        let cpm = s.cpm_at(now);
        let wpm = s.wpm_at(now);
        assert!(
            (wpm - cpm / 5.0).abs() < f32::EPSILON,
            "WPM should be CPM/5"
        );
    }

    // -- Streak tests --

    #[test]
    fn test_streak_starts_when_above_target() {
        let base = Instant::now();
        let mut s = make_state_at(100, base);
        // Push enough presses to exceed 100 CPM
        // 30 presses in 10 seconds = 180 CPM
        for i in 0..30 {
            s.press_times
                .push_back(base + Duration::from_millis(i * 333));
        }
        s.total_presses = 30;

        let now = base + Duration::from_secs(10);
        s.update_streak_at(now);
        assert!(s.above_target_since.is_some());
    }

    #[test]
    fn test_streak_resets_when_below_target() {
        let base = Instant::now();
        let mut s = make_state_at(100, base);
        // Set streak
        s.above_target_since = Some(base);

        // No presses → 0 CPM < 100
        let now = base + Duration::from_secs(5);
        s.update_streak_at(now);
        assert!(s.above_target_since.is_none());
    }

    #[test]
    fn test_streak_duration() {
        let base = Instant::now();
        let s = CoachState {
            config: CoachConfig {
                target_cpm: 100,
                show_heatmap: true,
                show_fingers: true,
            },
            total_presses: 0,
            backspace_count: 0,
            press_times: VecDeque::new(),
            session_start: base,
            key_counts: HashMap::new(),
            key_history: Vec::new(),
            max_count: 0,
            above_target_since: Some(base + Duration::from_secs(2)),
        };
        let now = base + Duration::from_secs(7);
        let secs = s.streak_secs_at(now);
        assert!((secs - 5.0).abs() < 0.01, "expected 5s streak, got {secs}");
    }

    #[test]
    fn test_streak_zero_when_not_above() {
        let s = make_state(200);
        assert!((s.streak_secs() - 0.0).abs() < f32::EPSILON);
    }

    // -- Layout tests --

    #[test]
    fn test_qwerty_layout_has_5_rows() {
        let layout = qwerty_layout();
        assert_eq!(layout.len(), 5);
    }

    #[test]
    fn test_qwerty_layout_rows_equal_width_units() {
        let layout = qwerty_layout();
        for (i, row) in layout.iter().enumerate() {
            let total: f32 = row.iter().map(|k| k.width).sum();
            assert!(
                (total - 15.0).abs() < 0.01,
                "row {i} sums to {total} units, expected 15.0"
            );
        }
    }

    #[test]
    fn test_qwerty_layout_contains_all_letters() {
        let layout = qwerty_layout();
        let all_labels: Vec<&str> = layout
            .iter()
            .flat_map(|row| row.iter().map(|k| k.label))
            .collect();
        for c in 'A'..='Z' {
            let s = String::from(c);
            assert!(
                all_labels.contains(&s.as_str()),
                "layout missing key '{s}'"
            );
        }
    }

    #[test]
    fn test_keyboard_width_positive() {
        let layout = qwerty_layout();
        let w = keyboard_width(&layout);
        assert!(w > 400.0, "keyboard width {w} unexpectedly small");
    }

    #[test]
    fn test_layout_finger_zones_valid() {
        let layout = qwerty_layout();
        for row in &layout {
            for k in row {
                assert!(
                    k.finger <= 8,
                    "key '{}' has invalid finger zone {}",
                    k.label,
                    k.finger
                );
            }
        }
    }

    // -- Color tests --

    #[test]
    fn test_heat_color_zero_count() {
        let c = heat_color(0, 10);
        assert_eq!(c, Color32::from_rgb(40, 40, 50));
    }

    #[test]
    fn test_heat_color_zero_max() {
        let c = heat_color(0, 0);
        assert_eq!(c, Color32::from_rgb(40, 40, 50));
    }

    #[test]
    fn test_heat_color_max_count() {
        let c = heat_color(10, 10);
        // Should be the hot end: (230, 55, 30)
        assert_eq!(c, Color32::from_rgb(230, 55, 30));
    }

    #[test]
    fn test_heat_color_mid_not_extremes() {
        let c = heat_color(5, 10);
        // At t=0.5, should be greenish (transition from cyan to green/yellow)
        let cold = Color32::from_rgb(40, 40, 50);
        let hot = Color32::from_rgb(230, 55, 30);
        assert_ne!(c, cold);
        assert_ne!(c, hot);
    }

    #[test]
    fn test_finger_colors_distinct() {
        let colors: Vec<Color32> = (0..=8).map(finger_color).collect();
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(
                    colors[i], colors[j],
                    "finger zones {i} and {j} have same color"
                );
            }
        }
    }

    // -- base_key tests --

    #[test]
    fn test_base_key_simple() {
        assert_eq!(base_key("A"), "A");
    }

    #[test]
    fn test_base_key_with_modifier() {
        assert_eq!(base_key("Ctrl+C"), "C");
    }

    #[test]
    fn test_base_key_multiple_modifiers() {
        assert_eq!(base_key("Ctrl+Shift+A"), "A");
    }

    #[test]
    fn test_base_key_backspace() {
        assert_eq!(base_key("Bksp"), "Bksp");
        assert_eq!(base_key("Ctrl+Bksp"), "Bksp");
    }

    // -- Config tests --

    #[test]
    fn test_coach_config_defaults() {
        let cfg = CoachConfig::default();
        assert_eq!(cfg.target_cpm, 200);
        assert!(cfg.show_heatmap);
        assert!(cfg.show_fingers);
    }

    // -- Prune tests --

    #[test]
    fn test_prune_old_presses() {
        let base = Instant::now();
        let mut s = make_state_at(200, base);
        // Add presses at t=0, 5, 12
        s.press_times.push_back(base);
        s.press_times
            .push_back(base + Duration::from_secs(5));
        s.press_times
            .push_back(base + Duration::from_secs(12));

        // At t=15, window is [5, 15]. t=0 should be pruned.
        let now = base + Duration::from_secs(15);
        s.prune_old_presses(now);
        assert_eq!(s.press_times.len(), 2);
    }

    // -- Integration-style tests --

    #[test]
    fn test_full_typing_session() {
        let base = Instant::now();
        let mut s = make_state_at(60, base);

        // Type "hello" then backspace once, then "o"
        let keys = ["H", "E", "L", "L", "Bksp", "O"];
        for (i, key) in keys.iter().enumerate() {
            let t = base + Duration::from_millis(i as u64 * 200);
            s.record_press_at(key, t);
        }

        assert_eq!(s.total_presses, 6);
        assert_eq!(s.backspace_count, 1);
        // accuracy = (6-1)/6 = 83.33%
        assert!((s.accuracy() - 83.333).abs() < 0.1);

        // heatmap: H=1, E=1, L=1 (was 2, backspace removed one), O=1
        assert_eq!(s.key_counts.get("H"), Some(&1));
        assert_eq!(s.key_counts.get("E"), Some(&1));
        assert_eq!(s.key_counts.get("L"), Some(&1));
        assert_eq!(s.key_counts.get("O"), Some(&1));
        assert_eq!(s.max_count, 1);

        // history should be ["H", "E", "L", "O"] (second L was removed by backspace)
        assert_eq!(s.key_history, vec!["H", "E", "L", "O"]);
    }

    #[test]
    fn test_modifier_presses_dont_inflate_stats() {
        let mut s = make_state(200);
        // In real usage, modifier-only presses are filtered by input.rs.
        // But combos like Ctrl+C come through. The base key is tracked.
        s.record_press("Ctrl+C");
        s.record_press("Ctrl+V");
        assert_eq!(s.key_counts.get("C"), Some(&1));
        assert_eq!(s.key_counts.get("V"), Some(&1));
        assert!(s.key_counts.get("Ctrl").is_none());
    }
}