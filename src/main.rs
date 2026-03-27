mod input;
mod overlay;

use clap::{Parser, Subcommand};
use crossbeam_channel::unbounded;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub font_size: f32,
    pub opacity: f32,
    pub display_time: f32,
    pub keys: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_size: 24.0,
            opacity: 0.75,
            display_time: 3.0,
            keys: 3,
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
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactively configure and save settings to ~/.config/keypop/config.toml
    Configure,
    /// Launch the keypress overlay using saved config
    Run,
}

fn cmd_configure() {
    let cur = load_config();
    println!("keypop configuration");
    println!("Press Enter to keep the value shown in [brackets].\n");

    let font_size = prompt_f32("Font size", cur.font_size, |v| v > 0.0, "must be > 0");
    let opacity = prompt_f32("Opacity (0.0–1.0)", cur.opacity, |v| (0.0..=1.0).contains(&v), "must be 0.0–1.0");
    let display_time = prompt_f32("Display time in seconds (2, 3, or 5)", cur.display_time, |v| v == 2.0 || v == 3.0 || v == 5.0, "must be 2, 3, or 5");
    let keys = prompt_u8("Number of keys to show (1–5)", cur.keys, |v| (1..=5).contains(&v), "must be 1–5");

    let cfg = Config { font_size, opacity, display_time, keys };
    match save_config(&cfg) {
        Ok(()) => println!("\nSaved to {}", config_path().display()),
        Err(e) => { eprintln!("Error saving config: {}", e); std::process::exit(1); }
    }
}

fn prompt_f32(label: &str, current: f32, validate: impl Fn(f32) -> bool, hint: &str) -> f32 {
    loop {
        print!("  {} [{}]: ", label, current);
        io::stdout().flush().unwrap();
        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        let s = line.trim();
        if s.is_empty() { return current; }
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
        if s.is_empty() { return current; }
        match s.parse::<u8>() {
            Ok(v) if validate(v) => return v,
            Ok(_) => println!("    Invalid: {}", hint),
            Err(_) => println!("    Invalid: not a number"),
        }
    }
}

fn cmd_run() {
    let cfg = load_config();
    let (tx, rx) = unbounded::<String>();

    let _input_thread = thread::Builder::new()
        .name("keypop-input".into())
        .spawn(move || {
            if let Err(e) = input::run(tx) {
                eprintln!("[keypop] input error: {}", e);
                eprintln!("[keypop] hint: add yourself to the 'input' group:");
                eprintln!("[keypop]   sudo usermod -aG input $USER  (then re-login)");
            }
        })
        .expect("failed to spawn input thread");

    overlay::run(cfg, rx);
}

fn main() {
    match Cli::parse().command {
        Commands::Configure => cmd_configure(),
        Commands::Run => cmd_run(),
    }
}
