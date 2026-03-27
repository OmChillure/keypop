use crossbeam_channel::Sender;
use evdev::{Device, EventType, Key};
use std::error::Error;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
extern crate libc;

/// Find all keyboard devices under /dev/input
fn find_keyboards() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let devices = evdev::enumerate();
    for (path, device) in devices {
        let keys_opt: Option<&evdev::AttributeSetRef<Key>> = device.supported_keys();
        if let Some(keys) = keys_opt {
            if keys.contains(Key::KEY_A) {
                found.push(path);
            }
        }
    }
    found
}

/// Convert an evdev Key into a human-readable display string
fn key_name(key: Key) -> Option<String> {
    let name = match key {
        // --- Modifiers ---
        Key::KEY_LEFTCTRL | Key::KEY_RIGHTCTRL => "Ctrl",
        Key::KEY_LEFTSHIFT | Key::KEY_RIGHTSHIFT => "Shift",
        Key::KEY_LEFTALT => "Alt",
        Key::KEY_RIGHTALT => "AltGr",
        Key::KEY_LEFTMETA | Key::KEY_RIGHTMETA => "Super",

        // --- Special keys ---
        Key::KEY_ESC => "Esc",
        Key::KEY_TAB => "Tab",
        Key::KEY_ENTER => "Enter",
        Key::KEY_BACKSPACE => "Bksp",
        Key::KEY_DELETE => "Del",
        Key::KEY_INSERT => "Ins",
        Key::KEY_HOME => "Home",
        Key::KEY_END => "End",
        Key::KEY_PAGEUP => "PgUp",
        Key::KEY_PAGEDOWN => "PgDn",
        Key::KEY_UP => "↑",
        Key::KEY_DOWN => "↓",
        Key::KEY_LEFT => "←",
        Key::KEY_RIGHT => "→",
        Key::KEY_SPACE => "Space",
        Key::KEY_CAPSLOCK => "Caps",
        Key::KEY_NUMLOCK => "NumLk",
        Key::KEY_SCROLLLOCK => "ScrLk",
        Key::KEY_SCREEN => "PrtSc",
        Key::KEY_PAUSE => "Pause",
        Key::KEY_COMPOSE => "Menu",

        // --- Function keys ---
        Key::KEY_F1 => "F1",
        Key::KEY_F2 => "F2",
        Key::KEY_F3 => "F3",
        Key::KEY_F4 => "F4",
        Key::KEY_F5 => "F5",
        Key::KEY_F6 => "F6",
        Key::KEY_F7 => "F7",
        Key::KEY_F8 => "F8",
        Key::KEY_F9 => "F9",
        Key::KEY_F10 => "F10",
        Key::KEY_F11 => "F11",
        Key::KEY_F12 => "F12",

        // --- Letters ---
        Key::KEY_A => "A",
        Key::KEY_B => "B",
        Key::KEY_C => "C",
        Key::KEY_D => "D",
        Key::KEY_E => "E",
        Key::KEY_F => "F",
        Key::KEY_G => "G",
        Key::KEY_H => "H",
        Key::KEY_I => "I",
        Key::KEY_J => "J",
        Key::KEY_K => "K",
        Key::KEY_L => "L",
        Key::KEY_M => "M",
        Key::KEY_N => "N",
        Key::KEY_O => "O",
        Key::KEY_P => "P",
        Key::KEY_Q => "Q",
        Key::KEY_R => "R",
        Key::KEY_S => "S",
        Key::KEY_T => "T",
        Key::KEY_U => "U",
        Key::KEY_V => "V",
        Key::KEY_W => "W",
        Key::KEY_X => "X",
        Key::KEY_Y => "Y",
        Key::KEY_Z => "Z",

        // --- Numbers row ---
        Key::KEY_1 => "1",
        Key::KEY_2 => "2",
        Key::KEY_3 => "3",
        Key::KEY_4 => "4",
        Key::KEY_5 => "5",
        Key::KEY_6 => "6",
        Key::KEY_7 => "7",
        Key::KEY_8 => "8",
        Key::KEY_9 => "9",
        Key::KEY_0 => "0",

        // --- Punctuation ---
        Key::KEY_MINUS => "-",
        Key::KEY_EQUAL => "=",
        Key::KEY_LEFTBRACE => "[",
        Key::KEY_RIGHTBRACE => "]",
        Key::KEY_BACKSLASH => "\\",
        Key::KEY_SEMICOLON => ";",
        Key::KEY_APOSTROPHE => "'",
        Key::KEY_GRAVE => "`",
        Key::KEY_COMMA => ",",
        Key::KEY_DOT => ".",
        Key::KEY_SLASH => "/",

        // --- Numpad ---
        Key::KEY_KP0 => "Num0",
        Key::KEY_KP1 => "Num1",
        Key::KEY_KP2 => "Num2",
        Key::KEY_KP3 => "Num3",
        Key::KEY_KP4 => "Num4",
        Key::KEY_KP5 => "Num5",
        Key::KEY_KP6 => "Num6",
        Key::KEY_KP7 => "Num7",
        Key::KEY_KP8 => "Num8",
        Key::KEY_KP9 => "Num9",
        Key::KEY_KPPLUS => "Num+",
        Key::KEY_KPMINUS => "Num-",
        Key::KEY_KPASTERISK => "Num*",
        Key::KEY_KPSLASH => "Num/",
        Key::KEY_KPDOT => "Num.",
        Key::KEY_KPENTER => "NumEnter",

        // --- Media keys ---
        Key::KEY_MUTE => "Mute",
        Key::KEY_VOLUMEUP => "Vol+",
        Key::KEY_VOLUMEDOWN => "Vol-",
        Key::KEY_PLAYPAUSE => "Play",
        Key::KEY_NEXTSONG => "Next",
        Key::KEY_PREVIOUSSONG => "Prev",
        Key::KEY_STOPCD => "Stop",

        // Ignore everything else (mouse moves, LEDs, etc.)
        _ => return None,
    };
    Some(name.to_string())
}

/// Track which modifier keys are currently held
#[derive(Default)]
struct ModState {
    ctrl: bool,
    shift: bool,
    alt: bool,
    super_: bool,
}

impl ModState {
    fn update(&mut self, key: Key, pressed: bool) {
        match key {
            Key::KEY_LEFTCTRL | Key::KEY_RIGHTCTRL => self.ctrl = pressed,
            Key::KEY_LEFTSHIFT | Key::KEY_RIGHTSHIFT => self.shift = pressed,
            Key::KEY_LEFTALT | Key::KEY_RIGHTALT => self.alt = pressed,
            Key::KEY_LEFTMETA | Key::KEY_RIGHTMETA => self.super_ = pressed,
            _ => {}
        }
    }

    fn is_modifier(key: Key) -> bool {
        matches!(
            key,
            Key::KEY_LEFTCTRL
                | Key::KEY_RIGHTCTRL
                | Key::KEY_LEFTSHIFT
                | Key::KEY_RIGHTSHIFT
                | Key::KEY_LEFTALT
                | Key::KEY_RIGHTALT
                | Key::KEY_LEFTMETA
                | Key::KEY_RIGHTMETA
        )
    }

    /// Build a display string like "Ctrl+Shift+A"
    fn format_with(&self, key_str: &str) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.super_ {
            parts.push("Super");
        }
        parts.push(key_str);
        parts.join("+")
    }
}

/// Open all keyboard devices and poll them in a round-robin loop.
/// Sends one string per keypress to the UI thread via `tx`.
pub fn run(tx: Sender<String>) -> Result<(), Box<dyn Error>> {
    let paths = find_keyboards();
    if paths.is_empty() {
        return Err(
            "no keyboard devices found in /dev/input — are you in the 'input' group?".into(),
        );
    }

    eprintln!("[keypop] found {} keyboard device(s):", paths.len());
    for p in &paths {
        eprintln!("[keypop]   {}", p.display());
    }

    // Open all devices
    let mut devices: Vec<Device> = paths
        .into_iter()
        .filter_map(|p| Device::open(&p).ok())
        .collect();

    if devices.is_empty() {
        return Err("failed to open any keyboard device — permission denied?".into());
    }

    // Set non-blocking so we can poll multiple devices
    for dev in &mut devices {
        unsafe {
            libc::fcntl(dev.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
        }
    }

    let mut mod_state = ModState::default();

    loop {
        let mut got_event = false;

        for dev in &mut devices {
            let Ok(events) = dev.fetch_events() else {
                continue;
            };

            for ev in events {
                if ev.event_type() != EventType::KEY {
                    continue;
                }

                let key = Key::new(ev.code());
                let value = ev.value();

                // value: 1 = pressed, 0 = released, 2 = repeat
                let pressed = value == 1;
                let repeated = value == 2;

                // Always update modifier state on press/release
                mod_state.update(key, pressed || repeated);

                if !pressed {
                    continue;
                }

                if ModState::is_modifier(key) {
                    continue;
                }

                if let Some(name) = key_name(key) {
                    let label = mod_state.format_with(&name);
                    if tx.send(label).is_err() {
                        return Ok(());
                    }
                    got_event = true;
                }
            }
        }

        if !got_event {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
