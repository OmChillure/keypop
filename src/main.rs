mod coach;
mod input;
mod overlay;
mod record;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub font_size: f32,
    pub opacity: f32,
    pub display_time: f32,
    pub keys: u8,
    #[serde(default = "default_record_dir")]
    pub record_dir: String,
    #[serde(default = "default_stop_hotkey")]
    pub stop_hotkey: String,
}

fn default_record_dir() -> String {
    dirs::video_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("Videos")
        })
        .join("Screencasts")
        .to_string_lossy()
        .into_owned()
}

fn default_stop_hotkey() -> String {
    "Ctrl+S".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_size: 24.0,
            opacity: 0.75,
            display_time: 3.0,
            keys: 3,
            record_dir: default_record_dir(),
            stop_hotkey: default_stop_hotkey(),
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("keypop")
        .join("config.toml")
}

fn load_config() -> Config {
    let path = config_path();
    if path.exists() {
        let text = fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&text).unwrap_or_default()
    } else {
        Config::default()
    }
}

fn save_config(cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, toml::to_string_pretty(cfg)?)?;
    Ok(())
}

/// keypop — transparent keypress overlay for Linux
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Configure,
    Run,
    /// Overlay + typing coach panel with live stats and keyboard heatmap
    Coach {
        /// Target CPM threshold for the streak counter
        #[arg(long, default_value = "200")]
        target_cpm: u32,
        /// Keyboard layout
        #[arg(long, default_value = "qwerty")]
        layout: String,
        /// Hide the keyboard heatmap
        #[arg(long)]
        no_heatmap: bool,
        /// Hide the keypress pill bar
        #[arg(long)]
        no_fingers: bool,
    },
    Record {
        #[command(subcommand)]
        action: RecordAction,
    },
}

#[derive(Subcommand)]
enum RecordAction {
    /// Start recording keystrokes with timing data
    Start {
        /// Override stop hotkey (default from config, e.g. "Ctrl+S")
        #[arg(long)]
        stop_hotkey: Option<String>,
    },
    /// Stop a running recording session
    Stop,
}

fn cmd_configure() {
    let cur = load_config();
    println!("keypop configuration");
    println!("Press Enter to keep the value shown in [brackets].\n");

    let font_size = prompt_f32("Font size", cur.font_size, |v| v > 0.0, "must be > 0");
    let opacity = prompt_f32(
        "Opacity (0.0–1.0)",
        cur.opacity,
        |v| (0.0..=1.0).contains(&v),
        "must be 0.0–1.0",
    );
    let display_time = prompt_f32(
        "Display time in seconds (2, 3, or 5)",
        cur.display_time,
        |v| v == 2.0 || v == 3.0 || v == 5.0,
        "must be 2, 3, or 5",
    );
    let keys = prompt_u8(
        "Number of keys to show (1–5)",
        cur.keys,
        |v| (1..=5).contains(&v),
        "must be 1–5",
    );

    let record_dir = prompt_string("Recording directory", &cur.record_dir);
    let stop_hotkey = prompt_string("Stop recording hotkey", &cur.stop_hotkey);

    let cfg = Config {
        font_size,
        opacity,
        display_time,
        keys,
        record_dir,
        stop_hotkey,
    };
    match save_config(&cfg) {
        Ok(()) => println!("\nSaved to {}", config_path().display()),
        Err(e) => {
            eprintln!("Error saving config: {}", e);
            std::process::exit(1);
        }
    }
}

fn prompt_f32(label: &str, current: f32, validate: impl Fn(f32) -> bool, hint: &str) -> f32 {
    loop {
        print!("  {} [{}]: ", label, current);
        io::stdout().flush().unwrap();
        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        let s = line.trim();
        if s.is_empty() {
            return current;
        }
        match s.parse::<f32>() {
            Ok(v) if validate(v) => return v,
            Ok(_) => println!("    Invalid: {}", hint),
            Err(_) => println!("    Invalid: not a number"),
        }
    }
}

fn prompt_u8(label: &str, current: u8, validate: impl Fn(u8) -> bool, hint: &str) -> u8 {
    loop {
        print!("  {} [{}]: ", label, current);
        io::stdout().flush().unwrap();
        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        let s = line.trim();
        if s.is_empty() {
            return current;
        }
        match s.parse::<u8>() {
            Ok(v) if validate(v) => return v,
            Ok(_) => println!("    Invalid: {}", hint),
            Err(_) => println!("    Invalid: not a number"),
        }
    }
}

fn prompt_string(label: &str, current: &str) -> String {
    loop {
        print!("  {} [{}]: ", label, current);
        io::stdout().flush().unwrap();
        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        let s = line.trim();
        if s.is_empty() {
            return current.to_string();
        }
        return s.to_string();
    }
}

fn cmd_run() {
    overlay::run(load_config());
}

fn cmd_coach(target_cpm: u32, show_heatmap: bool, show_fingers: bool) {
    let cfg = load_config();
    let coach_cfg = coach::CoachConfig {
        target_cpm,
        show_heatmap,
        show_fingers,
    };
    coach::run(cfg, coach_cfg);
}

fn cmd_record_start(stop_hotkey_override: Option<String>) {
    let mut cfg = load_config();
    if let Some(hk) = stop_hotkey_override {
        cfg.stop_hotkey = hk;
    }
    record::start(cfg);
}

fn cmd_record_stop() {
    record::stop();
}

fn cmd_menu() {
    println!("..K...K..EEEE..Y...Y..PPPP....OOO...PPPP..");
    println!("..K..K...E......Y.Y...P...P..O...O..P...P.");
    println!("..KKK....EEE.....Y....PPPP...O...O..PPPP..");
    println!("..K..K...E.......Y....P......O...O..P.....");
    println!("..K...K..EEEE....Y....P.......OOO...P.....");
    println!("-----------------------------------------------");
    println!("keypop --help");
    println!("keypop run");
    println!("keypop coach [--target-cpm 200] [--no-heatmap] [--no-fingers]");
    println!("keypop configure");
    println!("keypop record start [--stop-hotkey \"Ctrl+S\"]");
    println!("keypop record stop");
}

fn main() {
    match Cli::parse().command {
        Some(Commands::Configure) => cmd_configure(),
        Some(Commands::Run) => cmd_run(),
        Some(Commands::Coach {
            target_cpm,
            layout: _,
            no_heatmap,
            no_fingers,
        }) => cmd_coach(target_cpm, !no_heatmap, !no_fingers),
        Some(Commands::Record { action }) => match action {
            RecordAction::Start { stop_hotkey } => cmd_record_start(stop_hotkey),
            RecordAction::Stop => cmd_record_stop(),
        },
        None => cmd_menu(),
    }
}
