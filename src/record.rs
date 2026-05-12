use crate::input::{find_keyboards, key_name, ModState};
use crate::overlay::{KeyPopApp, RecordingIndicator};
use crate::Config;
use crossbeam_channel::Sender;
use evdev::{Device, EventType, Key};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
extern crate libc;

// ---------------------------------------------------------------------------
// Recording data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordedEvent {
    /// Human-readable key label, e.g. "A", "Ctrl+C"
    pub key: String,
    /// Milliseconds since recording started (kernel-level timing)
    pub time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recording {
    pub version: u32,
    /// Unix timestamp in milliseconds when recording started
    pub started_at_unix_ms: u64,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    pub events: Vec<RecordedEvent>,
}

// ---------------------------------------------------------------------------
// Hotkey parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_: bool,
    pub key: Key,
}

/// Map a single key name (case-insensitive) to an evdev Key.
pub fn string_to_key(s: &str) -> Option<Key> {
    Some(match s.to_uppercase().as_str() {
        "A" => Key::KEY_A,
        "B" => Key::KEY_B,
        "C" => Key::KEY_C,
        "D" => Key::KEY_D,
        "E" => Key::KEY_E,
        "F" => Key::KEY_F,
        "G" => Key::KEY_G,
        "H" => Key::KEY_H,
        "I" => Key::KEY_I,
        "J" => Key::KEY_J,
        "K" => Key::KEY_K,
        "L" => Key::KEY_L,
        "M" => Key::KEY_M,
        "N" => Key::KEY_N,
        "O" => Key::KEY_O,
        "P" => Key::KEY_P,
        "Q" => Key::KEY_Q,
        "R" => Key::KEY_R,
        "S" => Key::KEY_S,
        "T" => Key::KEY_T,
        "U" => Key::KEY_U,
        "V" => Key::KEY_V,
        "W" => Key::KEY_W,
        "X" => Key::KEY_X,
        "Y" => Key::KEY_Y,
        "Z" => Key::KEY_Z,
        "0" => Key::KEY_0,
        "1" => Key::KEY_1,
        "2" => Key::KEY_2,
        "3" => Key::KEY_3,
        "4" => Key::KEY_4,
        "5" => Key::KEY_5,
        "6" => Key::KEY_6,
        "7" => Key::KEY_7,
        "8" => Key::KEY_8,
        "9" => Key::KEY_9,
        "ESC" | "ESCAPE" => Key::KEY_ESC,
        "TAB" => Key::KEY_TAB,
        "ENTER" | "RETURN" => Key::KEY_ENTER,
        "SPACE" => Key::KEY_SPACE,
        "BACKSPACE" | "BKSP" => Key::KEY_BACKSPACE,
        "DELETE" | "DEL" => Key::KEY_DELETE,
        "INSERT" | "INS" => Key::KEY_INSERT,
        "HOME" => Key::KEY_HOME,
        "END" => Key::KEY_END,
        "PAGEUP" | "PGUP" => Key::KEY_PAGEUP,
        "PAGEDOWN" | "PGDN" => Key::KEY_PAGEDOWN,
        "UP" => Key::KEY_UP,
        "DOWN" => Key::KEY_DOWN,
        "LEFT" => Key::KEY_LEFT,
        "RIGHT" => Key::KEY_RIGHT,
        "F1" => Key::KEY_F1,
        "F2" => Key::KEY_F2,
        "F3" => Key::KEY_F3,
        "F4" => Key::KEY_F4,
        "F5" => Key::KEY_F5,
        "F6" => Key::KEY_F6,
        "F7" => Key::KEY_F7,
        "F8" => Key::KEY_F8,
        "F9" => Key::KEY_F9,
        "F10" => Key::KEY_F10,
        "F11" => Key::KEY_F11,
        "F12" => Key::KEY_F12,
        "-" | "MINUS" => Key::KEY_MINUS,
        "=" | "EQUAL" => Key::KEY_EQUAL,
        "[" => Key::KEY_LEFTBRACE,
        "]" => Key::KEY_RIGHTBRACE,
        "\\" | "BACKSLASH" => Key::KEY_BACKSLASH,
        ";" | "SEMICOLON" => Key::KEY_SEMICOLON,
        "'" | "APOSTROPHE" => Key::KEY_APOSTROPHE,
        "`" | "GRAVE" => Key::KEY_GRAVE,
        "," | "COMMA" => Key::KEY_COMMA,
        "." | "DOT" | "PERIOD" => Key::KEY_DOT,
        "/" | "SLASH" => Key::KEY_SLASH,
        _ => return None,
    })
}

/// Parse a hotkey string like "Ctrl+S", "Ctrl+Shift+R", "Alt+F4".
/// Returns `None` if the string is malformed.
pub fn parse_hotkey(s: &str) -> Option<Hotkey> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut hotkey = Hotkey {
        ctrl: false,
        alt: false,
        shift: false,
        super_: false,
        key: Key::KEY_RESERVED,
    };

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            hotkey.key = string_to_key(part)?;
        } else {
            match part.to_lowercase().as_str() {
                "ctrl" => hotkey.ctrl = true,
                "alt" | "altgr" => hotkey.alt = true,
                "shift" => hotkey.shift = true,
                "super" | "meta" | "win" => hotkey.super_ = true,
                _ => return None,
            }
        }
    }

    if hotkey.key == Key::KEY_RESERVED {
        return None;
    }
    Some(hotkey)
}

/// Check whether the current modifier state + pressed key matches a hotkey.
pub fn matches_hotkey(mod_state: &ModState, key: Key, hotkey: &Hotkey) -> bool {
    mod_state.ctrl == hotkey.ctrl
        && mod_state.alt == hotkey.alt
        && mod_state.shift == hotkey.shift
        && mod_state.super_ == hotkey.super_
        && key == hotkey.key
}

// ---------------------------------------------------------------------------
// PID file management
// ---------------------------------------------------------------------------

pub fn pid_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("keypop")
        .join("record.pid")
}

struct PidGuard {
    path: PathBuf,
}

impl PidGuard {
    fn create(path: PathBuf) -> Result<Self, Box<dyn Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, std::process::id().to_string())?;
        Ok(PidGuard { path })
    }
}

impl Drop for PidGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Signal handling (SIGUSR1 for external `record stop`)
// ---------------------------------------------------------------------------

static STOP_SIGNAL: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_stop_signal(_sig: libc::c_int) {
    STOP_SIGNAL.store(true, Ordering::SeqCst);
}

fn install_signal_handler() {
    unsafe {
        libc::signal(libc::SIGUSR1, handle_stop_signal as *const () as libc::sighandler_t);
    }
}

// ---------------------------------------------------------------------------
// Recording filename helpers
// ---------------------------------------------------------------------------

pub fn recording_dir(cfg: &Config) -> PathBuf {
    PathBuf::from(&cfg.record_dir)
}

pub fn generate_filename() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    unsafe {
        let time = now as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&time, &mut tm);
        format!(
            "keypop_{:04}{:02}{:02}_{:02}{:02}{:02}.json",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    }
}

pub fn save_recording(
    recording: &Recording,
    dir: &PathBuf,
    filename: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join(filename);
    let json = serde_json::to_string_pretty(recording)?;
    fs::write(&path, json)?;
    Ok(path)
}

// ---------------------------------------------------------------------------
// Recording input loop — reads /dev/input with kernel timestamps
// ---------------------------------------------------------------------------

/// Run the recording input loop. Sends key labels to `tx` for the overlay,
/// records events with kernel-accurate inter-event timing, and stops on
/// the configured stop hotkey or SIGUSR1 from `record stop`.
pub fn run_record_input(
    tx: Sender<String>,
    ctx: egui::Context,
    is_recording: Arc<AtomicBool>,
    stop_hotkey: &Hotkey,
) -> Result<Recording, Box<dyn Error>> {
    let paths = find_keyboards();
    if paths.is_empty() {
        return Err(
            "no keyboard devices found in /dev/input — are you in the 'input' group?".into(),
        );
    }

    eprintln!("[keypop] recording: found {} keyboard device(s)", paths.len());

    let mut devices: Vec<Device> = paths
        .into_iter()
        .filter_map(|p| Device::open(&p).ok())
        .collect();

    if devices.is_empty() {
        return Err("failed to open any keyboard device — permission denied?".into());
    }

    for dev in &mut devices {
        unsafe {
            libc::fcntl(dev.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
        }
    }

    let mut mod_state = ModState::default();
    let mut events: Vec<RecordedEvent> = Vec::new();
    let start_wall = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let start_instant = Instant::now();
    // The first kernel timestamp we see, used to compute deltas.
    let mut base_kernel_time: Option<SystemTime> = None;

    is_recording.store(true, Ordering::SeqCst);

    loop {
        // Check for external stop signal (from `keypop record stop`)
        if STOP_SIGNAL.load(Ordering::SeqCst) {
            break;
        }

        let mut got_event = false;

        for dev in &mut devices {
            let Ok(dev_events) = dev.fetch_events() else {
                continue;
            };

            for ev in dev_events {
                if ev.event_type() != EventType::KEY {
                    continue;
                }

                let key = Key::new(ev.code());
                let value = ev.value();

                let pressed = value == 1;
                let repeated = value == 2;

                mod_state.update(key, pressed || repeated);

                if !pressed {
                    continue;
                }

                if ModState::is_modifier(key) {
                    continue;
                }

                // Check stop hotkey BEFORE recording the event
                if matches_hotkey(&mod_state, key, stop_hotkey) {
                    is_recording.store(false, Ordering::SeqCst);
                    ctx.request_repaint();
                    let duration_ms = start_instant.elapsed().as_millis() as u64;
                    return Ok(Recording {
                        version: 1,
                        started_at_unix_ms: start_wall,
                        duration_ms,
                        events,
                    });
                }

                if let Some(name) = key_name(key) {
                    let label = mod_state.format_with(&name);

                    // Compute offset using kernel timestamp
                    let kernel_ts = ev.timestamp();
                    let offset_ms = match base_kernel_time {
                        Some(base) => kernel_ts
                            .duration_since(base)
                            .unwrap_or_else(|_| start_instant.elapsed())
                            .as_millis() as u64,
                        None => {
                            base_kernel_time = Some(kernel_ts);
                            0
                        }
                    };

                    events.push(RecordedEvent {
                        key: label.clone(),
                        time_ms: offset_ms,
                    });

                    if tx.send(label).is_err() {
                        // Overlay closed
                        break;
                    }
                    ctx.request_repaint();
                    got_event = true;
                }
            }
        }

        // Also check stop signal between iterations
        if STOP_SIGNAL.load(Ordering::SeqCst) {
            break;
        }

        if !got_event {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    is_recording.store(false, Ordering::SeqCst);
    ctx.request_repaint();
    let duration_ms = start_instant.elapsed().as_millis() as u64;
    Ok(Recording {
        version: 1,
        started_at_unix_ms: start_wall,
        duration_ms,
        events,
    })
}

// ---------------------------------------------------------------------------
// Public entry points: start / stop
// ---------------------------------------------------------------------------

/// Start recording: opens the overlay with a recording-aware input thread,
/// captures keystrokes with kernel timing, and saves to the configured dir.
pub fn start(cfg: Config) {
    let stop_hk = match parse_hotkey(&cfg.stop_hotkey) {
        Some(hk) => hk,
        None => {
            eprintln!(
                "[keypop] invalid stop hotkey '{}', falling back to Ctrl+S",
                cfg.stop_hotkey
            );
            parse_hotkey("Ctrl+S").unwrap()
        }
    };

    let rec_dir = recording_dir(&cfg);

    // Install SIGUSR1 handler for `record stop`
    install_signal_handler();

    // Write PID file
    let _pid_guard = match PidGuard::create(pid_file_path()) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[keypop] warning: could not write PID file: {e}");
            // Continue anyway — `record stop` just won't work
            PidGuard {
                path: PathBuf::new(),
            }
        }
    };

    eprintln!(
        "[keypop] recording started — press {} to stop",
        cfg.stop_hotkey
    );
    eprintln!("[keypop] recordings will be saved to {}", rec_dir.display());

    crate::overlay::run_with_input(cfg, move |tx, egui_ctx, args, rx, screen| {
        let is_recording = Arc::new(AtomicBool::new(false));
        let is_rec_clone = is_recording.clone();

        let indicator = RecordingIndicator {
            is_active: is_recording.clone(),
            started: Instant::now(),
        };

        let rec_dir_clone = rec_dir.clone();
        let ctx_clone = egui_ctx.clone();

        std::thread::Builder::new()
            .name("keypop-record".into())
            .spawn(move || {
                match run_record_input(tx, ctx_clone, is_rec_clone, &stop_hk) {
                    Ok(recording) => {
                        let filename = generate_filename();
                        match save_recording(&recording, &rec_dir_clone, &filename) {
                            Ok(path) => {
                                eprintln!("[keypop] recording saved: {}", path.display());
                                eprintln!(
                                    "[keypop] {} events, duration: {}ms",
                                    recording.events.len(),
                                    recording.duration_ms
                                );
                            }
                            Err(e) => eprintln!("[keypop] error saving recording: {e}"),
                        }
                    }
                    Err(e) => {
                        eprintln!("[keypop] recording error: {e}");
                        eprintln!("[keypop] hint: sudo usermod -aG input $USER  (then re-login)");
                    }
                }
            })
            .expect("failed to spawn recording thread");

        KeyPopApp::new(args, rx, screen).with_recording(indicator)
    });
}

/// Stop a running recording by sending SIGUSR1 to the process in the PID file.
pub fn stop() {
    let pid_path = pid_file_path();
    let pid_str = match fs::read_to_string(&pid_path) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("[keypop] no active recording found (no PID file at {})", pid_path.display());
            std::process::exit(1);
        }
    };

    let pid: i32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("[keypop] invalid PID in {}", pid_path.display());
            std::process::exit(1);
        }
    };

    // Check if process is alive
    let alive = unsafe { libc::kill(pid, 0) == 0 };
    if !alive {
        eprintln!("[keypop] recording process (PID {pid}) is not running — cleaning up stale PID file");
        let _ = fs::remove_file(&pid_path);
        std::process::exit(1);
    }

    // Send SIGUSR1 to stop recording
    let ret = unsafe { libc::kill(pid, libc::SIGUSR1) };
    if ret == 0 {
        eprintln!("[keypop] stop signal sent to recording process (PID {pid})");
    } else {
        eprintln!("[keypop] failed to send stop signal to PID {pid} — permission denied?");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Hotkey parsing tests --

    #[test]
    fn test_parse_hotkey_simple_key() {
        let hk = parse_hotkey("A").unwrap();
        assert!(!hk.ctrl);
        assert!(!hk.alt);
        assert!(!hk.shift);
        assert!(!hk.super_);
        assert_eq!(hk.key, Key::KEY_A);
    }

    #[test]
    fn test_parse_hotkey_ctrl_s() {
        let hk = parse_hotkey("Ctrl+S").unwrap();
        assert!(hk.ctrl);
        assert!(!hk.alt);
        assert!(!hk.shift);
        assert!(!hk.super_);
        assert_eq!(hk.key, Key::KEY_S);
    }

    #[test]
    fn test_parse_hotkey_ctrl_shift_r() {
        let hk = parse_hotkey("Ctrl+Shift+R").unwrap();
        assert!(hk.ctrl);
        assert!(!hk.alt);
        assert!(hk.shift);
        assert!(!hk.super_);
        assert_eq!(hk.key, Key::KEY_R);
    }

    #[test]
    fn test_parse_hotkey_alt_f4() {
        let hk = parse_hotkey("Alt+F4").unwrap();
        assert!(!hk.ctrl);
        assert!(hk.alt);
        assert!(!hk.shift);
        assert!(!hk.super_);
        assert_eq!(hk.key, Key::KEY_F4);
    }

    #[test]
    fn test_parse_hotkey_super_key() {
        let hk = parse_hotkey("Super+L").unwrap();
        assert!(!hk.ctrl);
        assert!(!hk.alt);
        assert!(!hk.shift);
        assert!(hk.super_);
        assert_eq!(hk.key, Key::KEY_L);
    }

    #[test]
    fn test_parse_hotkey_all_modifiers() {
        let hk = parse_hotkey("Ctrl+Alt+Shift+Super+A").unwrap();
        assert!(hk.ctrl);
        assert!(hk.alt);
        assert!(hk.shift);
        assert!(hk.super_);
        assert_eq!(hk.key, Key::KEY_A);
    }

    #[test]
    fn test_parse_hotkey_case_insensitive() {
        let hk = parse_hotkey("ctrl+shift+r").unwrap();
        assert!(hk.ctrl);
        assert!(hk.shift);
        assert_eq!(hk.key, Key::KEY_R);
    }

    #[test]
    fn test_parse_hotkey_with_spaces() {
        let hk = parse_hotkey("Ctrl + S").unwrap();
        assert!(hk.ctrl);
        assert_eq!(hk.key, Key::KEY_S);
    }

    #[test]
    fn test_parse_hotkey_invalid_key() {
        assert!(parse_hotkey("Ctrl+INVALID").is_none());
    }

    #[test]
    fn test_parse_hotkey_empty() {
        assert!(parse_hotkey("").is_none());
    }

    #[test]
    fn test_parse_hotkey_invalid_modifier() {
        assert!(parse_hotkey("Hyper+A").is_none());
    }

    #[test]
    fn test_parse_hotkey_numbers() {
        let hk = parse_hotkey("Ctrl+1").unwrap();
        assert!(hk.ctrl);
        assert_eq!(hk.key, Key::KEY_1);
    }

    #[test]
    fn test_parse_hotkey_function_keys() {
        for (s, expected) in [
            ("F1", Key::KEY_F1),
            ("F12", Key::KEY_F12),
        ] {
            let hk = parse_hotkey(s).unwrap();
            assert_eq!(hk.key, expected, "failed for {s}");
        }
    }

    #[test]
    fn test_parse_hotkey_special_keys() {
        for (s, expected) in [
            ("Escape", Key::KEY_ESC),
            ("Tab", Key::KEY_TAB),
            ("Enter", Key::KEY_ENTER),
            ("Space", Key::KEY_SPACE),
            ("Backspace", Key::KEY_BACKSPACE),
            ("Delete", Key::KEY_DELETE),
        ] {
            let hk = parse_hotkey(s).unwrap();
            assert_eq!(hk.key, expected, "failed for {s}");
        }
    }

    // -- Hotkey matching tests --

    #[test]
    fn test_matches_hotkey_ctrl_s() {
        let hk = parse_hotkey("Ctrl+S").unwrap();
        let mut ms = ModState::default();
        ms.ctrl = true;
        assert!(matches_hotkey(&ms, Key::KEY_S, &hk));
    }

    #[test]
    fn test_matches_hotkey_wrong_key() {
        let hk = parse_hotkey("Ctrl+S").unwrap();
        let mut ms = ModState::default();
        ms.ctrl = true;
        assert!(!matches_hotkey(&ms, Key::KEY_R, &hk));
    }

    #[test]
    fn test_matches_hotkey_missing_modifier() {
        let hk = parse_hotkey("Ctrl+S").unwrap();
        let ms = ModState::default(); // no ctrl held
        assert!(!matches_hotkey(&ms, Key::KEY_S, &hk));
    }

    #[test]
    fn test_matches_hotkey_extra_modifier_rejects() {
        let hk = parse_hotkey("Ctrl+S").unwrap();
        let mut ms = ModState::default();
        ms.ctrl = true;
        ms.shift = true; // extra modifier not in hotkey
        assert!(!matches_hotkey(&ms, Key::KEY_S, &hk));
    }

    #[test]
    fn test_matches_hotkey_no_modifiers() {
        let hk = parse_hotkey("A").unwrap();
        let ms = ModState::default();
        assert!(matches_hotkey(&ms, Key::KEY_A, &hk));
    }

    // -- string_to_key tests --

    #[test]
    fn test_string_to_key_letters() {
        assert_eq!(string_to_key("a"), Some(Key::KEY_A));
        assert_eq!(string_to_key("Z"), Some(Key::KEY_Z));
    }

    #[test]
    fn test_string_to_key_unknown() {
        assert_eq!(string_to_key("NOTAKEY"), None);
    }

    // -- Recording serialization tests --

    #[test]
    fn test_recording_roundtrip() {
        let rec = Recording {
            version: 1,
            started_at_unix_ms: 1700000000000,
            duration_ms: 5000,
            events: vec![
                RecordedEvent {
                    key: "H".to_string(),
                    time_ms: 0,
                },
                RecordedEvent {
                    key: "Ctrl+C".to_string(),
                    time_ms: 1500,
                },
            ],
        };

        let json = serde_json::to_string_pretty(&rec).unwrap();
        let deserialized: Recording = serde_json::from_str(&json).unwrap();
        assert_eq!(rec, deserialized);
    }

    #[test]
    fn test_recording_empty_events() {
        let rec = Recording {
            version: 1,
            started_at_unix_ms: 1700000000000,
            duration_ms: 0,
            events: vec![],
        };

        let json = serde_json::to_string(&rec).unwrap();
        let deserialized: Recording = serde_json::from_str(&json).unwrap();
        assert_eq!(rec, deserialized);
    }

    #[test]
    fn test_recording_preserves_timing_order() {
        let rec = Recording {
            version: 1,
            started_at_unix_ms: 1700000000000,
            duration_ms: 3000,
            events: vec![
                RecordedEvent { key: "A".into(), time_ms: 0 },
                RecordedEvent { key: "B".into(), time_ms: 150 },
                RecordedEvent { key: "C".into(), time_ms: 280 },
                RecordedEvent { key: "Ctrl+S".into(), time_ms: 3000 },
            ],
        };
        for w in rec.events.windows(2) {
            assert!(w[0].time_ms <= w[1].time_ms, "events not in order");
        }
    }

    // -- File save tests --

    #[test]
    fn test_save_recording_creates_file() {
        let dir = std::env::temp_dir().join("keypop_test_save");
        let _ = fs::remove_dir_all(&dir);

        let rec = Recording {
            version: 1,
            started_at_unix_ms: 1700000000000,
            duration_ms: 100,
            events: vec![RecordedEvent {
                key: "A".into(),
                time_ms: 0,
            }],
        };

        let path = save_recording(&rec, &dir, "test_rec.json").unwrap();
        assert!(path.exists());

        let content = fs::read_to_string(&path).unwrap();
        let loaded: Recording = serde_json::from_str(&content).unwrap();
        assert_eq!(rec, loaded);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_recording_creates_nested_dir() {
        let dir = std::env::temp_dir()
            .join("keypop_test_nested")
            .join("deep")
            .join("dir");
        let _ = fs::remove_dir_all(std::env::temp_dir().join("keypop_test_nested"));

        let rec = Recording {
            version: 1,
            started_at_unix_ms: 1700000000000,
            duration_ms: 0,
            events: vec![],
        };

        let path = save_recording(&rec, &dir, "nested.json").unwrap();
        assert!(path.exists());

        let _ = fs::remove_dir_all(std::env::temp_dir().join("keypop_test_nested"));
    }

    // -- Filename generation test --

    #[test]
    fn test_generate_filename_format() {
        let name = generate_filename();
        assert!(name.starts_with("keypop_"));
        assert!(name.ends_with(".json"));
        // Should match pattern keypop_YYYYMMDD_HHMMSS.json
        assert_eq!(name.len(), "keypop_YYYYMMDD_HHMMSS.json".len());
    }

    // -- PID file path test --

    #[test]
    fn test_pid_file_path_is_deterministic() {
        let a = pid_file_path();
        let b = pid_file_path();
        assert_eq!(a, b);
        assert!(a.ends_with("keypop/record.pid"));
    }

    // -- ModState interaction tests --

    #[test]
    fn test_mod_state_update_and_format() {
        let mut ms = ModState::default();
        ms.update(Key::KEY_LEFTCTRL, true);
        assert!(ms.ctrl);
        assert_eq!(ms.format_with("C"), "Ctrl+C");

        ms.update(Key::KEY_LEFTSHIFT, true);
        assert_eq!(ms.format_with("A"), "Ctrl+Shift+A");

        ms.update(Key::KEY_LEFTCTRL, false);
        assert!(!ms.ctrl);
        assert_eq!(ms.format_with("A"), "Shift+A");
    }

    #[test]
    fn test_mod_state_is_modifier() {
        assert!(ModState::is_modifier(Key::KEY_LEFTCTRL));
        assert!(ModState::is_modifier(Key::KEY_RIGHTCTRL));
        assert!(ModState::is_modifier(Key::KEY_LEFTSHIFT));
        assert!(ModState::is_modifier(Key::KEY_RIGHTSHIFT));
        assert!(ModState::is_modifier(Key::KEY_LEFTALT));
        assert!(ModState::is_modifier(Key::KEY_RIGHTALT));
        assert!(ModState::is_modifier(Key::KEY_LEFTMETA));
        assert!(ModState::is_modifier(Key::KEY_RIGHTMETA));
        assert!(!ModState::is_modifier(Key::KEY_A));
        assert!(!ModState::is_modifier(Key::KEY_SPACE));
    }

    // -- Config backward compatibility tests --

    #[test]
    fn test_config_deserialize_without_new_fields() {
        // Old config file without record_dir / stop_hotkey should still parse
        let toml_str = r#"
            font_size = 28.0
            opacity = 0.9
            display_time = 5.0
            keys = 4
        "#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.font_size, 28.0);
        assert_eq!(cfg.keys, 4);
        // New fields should have their defaults
        assert_eq!(cfg.stop_hotkey, "Ctrl+S");
        assert!(!cfg.record_dir.is_empty());
    }

    #[test]
    fn test_config_deserialize_with_new_fields() {
        let toml_str = r#"
            font_size = 24.0
            opacity = 0.75
            display_time = 3.0
            keys = 3
            record_dir = "/custom/path"
            stop_hotkey = "Ctrl+Shift+Q"
        "#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.record_dir, "/custom/path");
        assert_eq!(cfg.stop_hotkey, "Ctrl+Shift+Q");
    }

    #[test]
    fn test_config_roundtrip() {
        let cfg = Config::default();
        let serialized = toml::to_string_pretty(&cfg).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(cfg.font_size, deserialized.font_size);
        assert_eq!(cfg.opacity, deserialized.opacity);
        assert_eq!(cfg.display_time, deserialized.display_time);
        assert_eq!(cfg.keys, deserialized.keys);
        assert_eq!(cfg.record_dir, deserialized.record_dir);
        assert_eq!(cfg.stop_hotkey, deserialized.stop_hotkey);
    }

    // -- Recording indicator flag test --

    #[test]
    fn test_recording_flag_atomic_operations() {
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        assert!(!flag.load(Ordering::SeqCst));

        flag_clone.store(true, Ordering::SeqCst);
        assert!(flag.load(Ordering::SeqCst));

        flag_clone.store(false, Ordering::SeqCst);
        assert!(!flag.load(Ordering::SeqCst));
    }

    // -- Stop signal flag test --

    #[test]
    fn test_stop_signal_flag() {
        // Reset the global flag
        STOP_SIGNAL.store(false, Ordering::SeqCst);
        assert!(!STOP_SIGNAL.load(Ordering::SeqCst));

        STOP_SIGNAL.store(true, Ordering::SeqCst);
        assert!(STOP_SIGNAL.load(Ordering::SeqCst));

        // Reset for other tests
        STOP_SIGNAL.store(false, Ordering::SeqCst);
    }
}