# RCast

A local-first, privacy-focused podcast client for desktop. No accounts. No algorithms. No cloud. Just your podcasts.

![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-lightgrey)
![License](https://img.shields.io/badge/license-MIT-blue)
![Version](https://img.shields.io/badge/version-0.1.1-green)

---

## Philosophy

RCast is built around a simple belief: your listening habits are your own business. Most podcast apps treat your
behaviour as data to be harvested — what you play, how long you listen, what you skip. RCast does none of that.

Everything lives on your machine. Your subscriptions, your playback position, your notes and bookmarks — all stored in a
local SQLite database that you own and can take with you. No server ever sees what you're listening to.

There are no recommendation algorithms. RCast will not suggest podcasts based on what other people listen to, will not
surface trending content, and will not try to maximise your time in the app. You decide what you listen to.

---

## Features

- **Podcast subscriptions** — Add any podcast by RSS feed URL. Automatic background sync keeps episodes fresh.
- **Playback** — Full audio controls with seek, skip forward/backward, and per-session speed control. Remembers your
  position so you can pick up exactly where you left off.
- **Episode queue** — Build a listening queue across any of your subscriptions.
- **Notes & bookmarks** — Write notes against any episode, optionally stamped to a timestamp. Clickable timestamps seek
  directly to that moment. Three note types: podcast-level, timed episode, and general episode notes.
- **OPML import/export** — Move your subscriptions in and out of RCast in the standard podcast interchange format.
  Compatible with any other podcast app.
- **Grid and list views** — Switch between artwork grid and compact list on the home screen. Preference is saved.
- **Played state tracking** — Episodes are visually distinguished once played. Mark as played or unplayed at any time.
- **Sync status** — Every podcast card shows when it was last synced. A live spinner appears during sync.

**What RCast intentionally does not do:**

- No user accounts or cloud sync
- No recommendation algorithms or trending content
- No telemetry, analytics, or usage tracking
- No ads

---

## Installation

### Download a release (recommended)

Download the latest pre-built binary for your platform from the [Releases](../../releases) page.

- **Windows** — download `rcast-windows.exe`, place it anywhere, run it.
- **macOS** — download `rcast-macos`, move it to your Applications folder. On first launch you may need to right-click →
  Open to bypass Gatekeeper.

### Build from source

**Prerequisites:**

- [Rust](https://rustup.rs/) 1.80 or later
- Windows: no additional dependencies
- macOS: Xcode Command Line Tools (`xcode-select --install`)

```bash
git clone https://github.com/your-username/rcast.git
cd rcast
cargo build --release
```

The binary will be at `target/release/rcast` (or `rcast.exe` on Windows).

---

## Architecture

RCast is written in Rust using [egui](https://github.com/emilk/egui)
via [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) for the UI. All state is persisted to a local
SQLite database via [rusqlite](https://github.com/rusqlite/rusqlite).

The application follows a ports-and-adapters (hexagonal) architecture:

```
src/
├-- application.rs       # Top-level eframe App, wires everything together
├-- orchestrator.rs      # All business logic and async command handling
├-- state.rs             # Shared UI state, updated by events from the orchestrator
├-- commands.rs          # UI → orchestrator messages
├-- events.rs            # Orchestrator → UI messages
├-- db/                  # Database layer (SQLite via rusqlite)
├-- ports/               # Abstract interfaces (FolderPicker, FilePicker)
├-- adapters/            # Concrete implementations (rfd dialogs)
├-- components/          # Reusable UI components
├-- pages/               # Full-page views (Home, PodcastDetail, Settings)
├-- migrations/          # Database schema migrations
└-- types.rs             # Shared domain types
```

The UI never touches the database directly. Pages and components dispatch `AppCommand` messages, the orchestrator
processes them asynchronously, and results flow back as `AppEvent` messages that update `AppState` each frame.

---

## Data

All application data is stored in your OS's local data directory:

- **Windows:** `%LOCALAPPDATA%\rcast\`
- **macOS:** `~/Library/Application Support/rcast/`

The database file is `rcast.db`. It is a standard SQLite database — you can open it with any SQLite browser to inspect
or export your data.

---

## Contributing

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for how to contribute and [RELEASING.md](docs/RELEASING.md) for the release
process.

Please open an issue before starting significant work so we can discuss the approach first.

---

## License

MIT. See [LICENSE](LICENSE).