# Contributing to RCast

Thanks for your interest in contributing. RCast is a small project with a clear philosophy — local-first,
privacy-focused, no algorithms — and contributions should fit within that.

---

## Before you start

Please open an issue before starting significant work. This avoids situations where you invest time in something that
doesn't fit the project's direction or that someone else is already working on.

For small bug fixes or obvious improvements, a PR without a prior issue is fine.

---

## Development setup

**Prerequisites:**

- [Rust](https://rustup.rs/) 1.80 or later
- Windows or macOS (Linux builds are not currently distributed but the code compiles)

```bash
git clone https://github.com/your-username/rcast.git
cd rcast
cargo run
```

The application will create its database in your OS's local data directory on first run.

---

## Project structure

```
src/
├-- application.rs    # eframe App, update loop, wires pages + components
├-- orchestrator.rs   # All async business logic, database calls
├-- state.rs          # AppState — shared UI state updated by events
├-- commands.rs       # UI → orchestrator (AppCommand enum)
├-- events.rs         # Orchestrator → UI (AppEvent enum)
├-- db/               # Database layer
├-- ports/            # Abstract interfaces (trait objects)
├-- adapters/         # Concrete implementations of ports
├-- components/       # Reusable UI pieces
├-- pages/            # Full-page views
├-- migrations/       # Database schema migrations
└-- types.rs          # Shared domain types
```

**Key principle:** Pages and components never touch the database. They dispatch `AppCommand` messages. The orchestrator
processes them and sends `AppEvent` responses. `application.rs` applies events to `AppState` every frame.

---

## Making changes

### Database schema changes

Every schema change needs a migration:

1. Create `src/migrations/versions/your_migration_name.rs`
2. Implement the `Migration` trait with `up()` and `down()`
3. Register it in `src/migrations/versions/mod.rs`
4. Add it to all three lists in `src/migrations/mod.rs`

Migrations run automatically on startup. The `down()` function must be implemented even if it's rarely used.

### Adding a new command

1. Add a variant to `AppCommand` in `commands.rs`
2. Handle it in `orchestrator.rs`
3. Fire the appropriate `AppEvent` from the handler
4. Handle the event in `application.rs` (`handle_event`)

### Adding a new setting

1. Add the field to `Settings` in `types.rs` with a sensible default in `Default`
2. Add the key to `get_settings` and `save_settings` in `db/mod.rs`
3. Add the UI control in `pages/settings.rs`

---

## Pull request process

Use the appropriate PR template:

- **Feature or bug fix:** use the [feature/bug template](../.github/PULL_REQUEST_TEMPLATE/feature_bug.md)
- **Release:** use the [release template](../.github/PULL_REQUEST_TEMPLATE/release.md)

CI will run automatically on PRs that touch `src/`, `Cargo.toml`, or `Cargo.lock`. It must pass before merging.

---

## Release process

See [RELEASING.md](RELEASING.md). Only admins can publish releases.