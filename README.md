# Rcast

A privacy-focused desktop podcast player built with Rust. This project was created as a learning exercise to explore
systems programming while building something useful that respects user privacy—no telemetry, no data collection, just
podcasts.

## Features

- **Privacy First**: No analytics, tracking, or external data collection
- **Local-First Architecture**: All data stored locally on your device
- ~~**OPML Import/Export**: Easily migrate your podcast subscriptions~~ (Not yet implemented)
- **Episode Management**: Download, stream, and organize episodes
- **Playback Controls**: Speed adjustment, skip intervals, and resume position tracking
- **Cross-Platform**: Runs on Windows, macOS, and Linux

## Tech Stack

- **Language**: Rust
- **GUI Framework**: [egui](https://github.com/emilk/egui)
- **Audio**: [rodio](https://github.com/RustAudio/rodio)
- **RSS Parsing**: [rss](https://crates.io/crates/rss)

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Cargo](https://doc.rust-lang.org/cargo/)

### Installation

```bash
# Clone the repository
git clone https://github.com/nickmorell/rcast.git
cd rcast

# Build release binary
cargo build --release

# Run in development mode
cargo run
