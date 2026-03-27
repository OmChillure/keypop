# [keypop](https://crates.io/crates/keypop)

A transparent keypress overlay for Linux. Shows your recent keypresses in a pill bar — great for screencasts, tutorials, and live demos.

Works on **X11** and **Wayland**.

```
┌──────────────────────────────────┐
│                                  │
│           (your desktop)         │
│                                  │
│              ┌──────────────────┐│
│              │ Ctrl+C │ Tab │ V ││
│              └──────────────────┘│
│                               ▲  │
│              ════════════════════│
└──────────────────────────────────┘
```

---

## Install

Make sure you have [Rust](https://rustup.rs) installed, then:

```bash
cargo install --git https://github.com/OmChillure/keypop
```

This builds a release binary and places it in `~/.cargo/bin/keypop`.

### System dependencies [if not already]

```bash
# Ubuntu / Debian
sudo apt install libxkbcommon-dev libwayland-dev pkg-config

# Fedora
sudo dnf install libxkbcommon-devel wayland-devel
```

### Permissions

keypop reads directly from `/dev/input`. Add yourself to the `input` group — no root required:

```bash
sudo usermod -aG input $USER
# log out and back in, then verify:
groups | grep input
```

---

## Usage

### Configure

Set your preferences interactively. Press **Enter** to keep the value shown in brackets:

```bash
keypop configure
```

```
keypop configuration
Press Enter to keep the value shown in [brackets].

  Font size [24]: 28
  Opacity (0.0–1.0) [0.75]:
  Display time in seconds (2, 3, or 5) [3]: 5
  Number of keys to show (1–5) [3]:

Saved to /home/user/.config/keypop/config.toml
```

Settings are saved to `~/.config/keypop/config.toml`. Run `keypop configure` again at any time to update them.

### Run

```bash
keypop run
```

Press **Esc** or **Ctrl+C** to quit.

### Command list

```bash
keypop
```

Shows:

```text
..K...K..EEEE..Y...Y..PPPP....OOO...PPPP..
..K..K...E......Y.Y...P...P..O...O..P...P.
..KKK....EEE.....Y....PPPP...O...O..PPPP..
..K..K...E.......Y....P......O...O..P.....
..K...K..EEEE....Y....P.......OOO...P.....
-----------------------------------------------
keypop --help
keypop run
keypop configure
```

---

## Options

| Setting | Default | Description |
|---------|---------|-------------|
| `font_size` | `24` | Key label font size in pixels |
| `opacity` | `0.75` | Bar opacity (0.0–1.0) |
| `display_time` | `3` | Seconds before overlay hides (2, 3, or 5) |
| `keys` | `3` | Number of recent keys to show (1–5) |

---

## Tiling WM notes

| WM | Config |
|----|--------|
| **Sway** | `for_window [app_id="keypop"] floating enable, sticky enable` |
| **Hyprland** | `windowrulev2 = float, class:^(keypop)$` + `windowrulev2 = pin, class:^(keypop)$` |
| **i3** | Requires a compositor (e.g. `picom`) for transparency |
| **GNOME / KDE** | Works out of the box |

---

## Contributing

1. Create an issue for the bug or feature.
2. Fix it in your branch.
3. Open a pull request linked to the issue.

---

## Socials

- X: https://x.com/OmChillure
- LinkedIn: https://www.linkedin.com/in/omchillure
- GitHub: https://github.com/OmChillure/keypop

---

## License

MIT — see [LICENSE](LICENSE)
