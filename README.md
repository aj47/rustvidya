# RustVidya 🎬

<img width="1192" height="1059" alt="image" src="https://github.com/user-attachments/assets/e144c897-3600-4ba1-8a76-b64d3c3cdc95" />


A terminal-based video player and webcam viewer that renders video as braille characters using the [dotmax](https://crates.io/crates/dotmax) crate.

![Rust](https://img.shields.io/badge/rust-stable-orange.svg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 🎥 **Video Playback** - Play video files rendered as braille in the terminal
- 📷 **Webcam Streaming** - Live webcam feed rendered in real-time
- 🎨 **Multiple Render Modes** - Braille, ASCII, Simple, and Block characters
- 🌈 **Color Schemes** - Rainbow, HeatMap, BluePurple, GreenYellow, CyanMagenta, Grayscale
- 🖼️ **Dithering Algorithms** - Floyd-Steinberg, Atkinson, Bayer, or None
- 📁 **File Browser** - Navigate and select video files with a TUI interface

## Installation

### Prerequisites

- Rust (stable)
- FFmpeg (for video/webcam capture)

### Build

```bash
git clone https://github.com/aj47/rustvidya.git
cd rustvidya
cargo build --release
```

### Run

```bash
cargo run --release
```

## Keyboard Controls

### General

| Key | Action |
|-----|--------|
| `q` | Quit |
| `w` | Toggle between Video and Webcam mode |
| `↑/k` | Navigate up |
| `↓/j` | Navigate down |
| `Backspace` | Go to parent directory |

### Video Mode

| Key | Action |
|-----|--------|
| `Enter` | Play selected video |
| `Space` | Play/Pause |
| `←/h` | Seek backward |
| `→/l` | Seek forward |
| `s` | Stop playback |

### Webcam Mode

| Key | Action |
|-----|--------|
| `Space` | Toggle streaming on/off |
| `s` | Stop streaming |
| `n` | Switch to next webcam device |
| `m` | Cycle render mode (Braille → ASCII → Simple → Blocks) |
| `c` | Cycle color scheme |
| `d` | Cycle dithering mode (None → Floyd-Steinberg → Atkinson → Bayer) |
| `+/-` | Adjust brightness threshold |

## Render Modes

| Mode | Description |
|------|-------------|
| **Braille** | Binary braille dots (2×4 dots per cell) |
| **ASCII** | 69-character gradient for smooth shading |
| **Simple** | 10-character simple density |
| **Blocks** | Unicode block characters |

## Dithering Algorithms

| Algorithm | Description |
|-----------|-------------|
| **None** | Simple threshold (fast, high contrast) |
| **Floyd-Steinberg** | Error diffusion for smooth gradients |
| **Atkinson** | High contrast dithering (Mac-style) |
| **Bayer** | Ordered 4×4 matrix dithering (patterned) |

## Color Schemes

- **None** - White/monochrome
- **Rainbow** - Red → Orange → Yellow → Green → Blue → Purple
- **HeatMap** - Black → Red → Orange → Yellow → White
- **BluePurple** - Blue → Purple gradient
- **GreenYellow** - Green → Yellow gradient
- **CyanMagenta** - Cyan → Magenta gradient
- **Grayscale** - Black → White gradient

## Screenshots

```
📷 FaceTime HD Camera | Mode: Braille | Dither: Floyd-Steinberg | Color: None | Thresh: 80 | 30fps
┌─────────────────────────────────────────────────────────────────────────────┐
│⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀│
│⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣴⣶⣶⣶⣦⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀│
│⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣾⣿⣿⣿⣿⣿⣿⣿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀│
└─────────────────────────────────────────────────────────────────────────────┘
```

## Dependencies

- [ratatui](https://crates.io/crates/ratatui) - Terminal UI framework
- [crossterm](https://crates.io/crates/crossterm) - Cross-platform terminal manipulation
- [dotmax](https://crates.io/crates/dotmax) - Braille rendering library
- [walkdir](https://crates.io/crates/walkdir) - Directory traversal
- [anyhow](https://crates.io/crates/anyhow) - Error handling

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [dotmax](https://github.com/newjordan/dotmax) by newjordan for the braille rendering library
- Inspired by terminal-based media players and ASCII art generators

