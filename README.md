# Mun

Alfred-style launcher and Rectangle-like window tiling utility for Linux and macOS.

Built with [egui](https://github.com/emilk/egui).

## Features

**Launcher** (`Ctrl+Space` by default)
- Fuzzy search for installed applications
- Web search directly from the launcher
- Browser bookmark search
- Built-in calculator (prefix with `=`)
- Usage history with frecency-style ranking

**Window Tiling** (`Alt+Ctrl` + arrow keys by default)
- Half-screen snapping (left, right, top, bottom)
- Quarter-screen snapping (four corners)
- Sixth-screen snapping (six zones)
- Maximize and center

## Installation

Download the latest release for your platform from the [releases page](../../releases).

### Build from source

```sh
cargo build --release
```

Linux requires: `libxcb-shape0-dev`, `libxcb-xfixes0-dev`, `libxkbcommon-dev`, `libssl-dev`, `libgtk-3-dev`, `libclang-dev`.

## Configuration

Config is stored at:

| Platform | Path |
|----------|------|
| Linux    | `~/.config/mun/config.json` |
| macOS    | `~/Library/Application Support/mun/config.json` |

All hotkeys are configurable. Example:

```json
{
  "launcher_hotkey": {
    "modifiers": ["Ctrl"],
    "key": "Space"
  },
  "window_actions": {
    "LeftHalf": { "modifiers": ["Alt", "Ctrl"], "key": "Left" },
    "RightHalf": { "modifiers": ["Alt", "Ctrl"], "key": "Right" },
    "Maximize": { "modifiers": ["Alt", "Ctrl"], "key": "Enter" }
  }
}
```

### Default window tiling hotkeys

| Action | Key |
|--------|-----|
| Left half | `Alt+Ctrl+Left` |
| Right half | `Alt+Ctrl+Right` |
| Top half | `Alt+Ctrl+Up` |
| Bottom half | `Alt+Ctrl+Down` |
| Top-left quarter | `Alt+Ctrl+1` |
| Top-right quarter | `Alt+Ctrl+2` |
| Bottom-left quarter | `Alt+Ctrl+3` |
| Bottom-right quarter | `Alt+Ctrl+4` |
| Top-left sixth | `Alt+Ctrl+Q` |
| Top-center sixth | `Alt+Ctrl+W` |
| Top-right sixth | `Alt+Ctrl+E` |
| Bottom-left sixth | `Alt+Ctrl+A` |
| Bottom-center sixth | `Alt+Ctrl+S` |
| Bottom-right sixth | `Alt+Ctrl+D` |
| Maximize | `Alt+Ctrl+Enter` |
| Center | `Alt+Ctrl+C` |

## License

All rights reserved.
