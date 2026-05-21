# Icon Asset Manifest

This directory contains icon assets for the RCast application across multiple platforms.

## File Usage

### Platform-Specific Icons

| File | Platform | Use Case | Size |
|------|----------|----------|------|
| `icon.ico` | Windows | Task manager, taskbar, executable | Multi-size (16–256px) |
| `icon.icns` | macOS | Dock, Finder, menu bar | Multi-size (16–1024px) |
| `icon-256x256.png` | Windows/macOS/Linux | Application window (eframe), taskbar icon | 256×256 px |
| `icon-128x128.png` | All platforms | System tray icon fallback | 128×128 px |
| `icon-32x32.png` | All platforms | Small UI elements, bookmarks | 32×32 px |
| `icon-16x16.png` | All platforms | Favicon, minimal context | 16×16 px |

### Scalable Variants

| File | Use Case | Background | Bars |
|------|----------|-----------|------|
| `icon-dark.svg` | Primary variant (dark surfaces) | `#120D0B` | `#CE422B` |
| `icon-light.svg` | Light background contexts | `#F5EFE8` | `#CE422B` |
| `icon-mono.svg` | Print, embossing, constraints | `#1E1E1E` | `#FFFFFF` |

### Raster Sizes

PNG files available at:
- 16×16 px (minimal, favicon)
- 32×32 px (taskbar small, preferences)
- 48×48 px (context menu, small UI)
- 64×64 px (launcher, application menu)
- 128×128 px (system tray standard)
- 256×256 px (window icon, standard export)
- 512×512 px (high-DPI, future use)
- 1024×1024 px (very high-DPI, archive)

## Integration Points

### cargo-bundle Configuration
- **Windows:** Uses `icon.ico` from Cargo.toml `[package.metadata.bundle.windows]`
- **macOS:** Uses `icon.icns` from Cargo.toml `[package.metadata.bundle.osx]`
- **Generic:** Uses PNG icons listed in `[package.metadata.bundle] icon = [...]`

### Rust Application
- **Window icon** (eframe): `icon-256x256.png` loaded in `src/main.rs`
- **System tray**: `icon-128x128.png` loaded in `src/tray.rs`

## Build System

The `build.rs` script validates that all required icon files exist at compile time and triggers a rebuild if any file in this directory changes.

## Notes

- **Minimum recommended size:** 16×16 px (do not render smaller)
- **Clear space:** ½ × icon height on all four sides
- **Waveform design:** Five vertical bars (asymmetric height, bottom-aligned)
  - Bar heights: 22, 44, 60, 46, 20 SVG units
  - Opacity: 0.42, 0.78, 1.00, 0.70, 0.38
- **Color rationale:** `#CE422B` (Rust programming language brand color)
