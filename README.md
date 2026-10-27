# KDE Dynamic Island

A premium, native Dynamic Island for **KDE Plasma 6 (Wayland)** built from the ground up in **Rust** and **Slint**.

## Features & Highlights

- **Pure Rust Architecture**: Clean, modular monolith with zero legacy C++/QML baggage.
- **Slint UI & Hardware Acceleration**: Sub-millisecond rendering with OpenGL/FemtoVG.
- **KDE Plasma 6 Native Wayland Integration**:
  - Pinned directly to the top edge of the display with transparent borders and smooth concave corners.
  - Active window overlap detection via native `org_kde_plasma_window_management` Wayland protocol.
- **4 MVP Views**:
  1. **Compact View**: Minimalist pill with 3D Pet sprite, digital clock, and real-time CAVA audio visualizer / charging status.
  2. **Idle View**: Expanded dashboard with 3D Pet, clock, date, battery level, divider, and quick action buttons.
  3. **Idle Music View**: Seamless MPRIS media player integration displaying track title, artist, playback status, and media control buttons.
  4. **Welcome View**: Dynamic entrance greeting on system startup or unlock, automatically morphing to standard views after 3.5 seconds.

## Architecture

- `apps/island-app`: Bootstrap, wiring, runtime loop, and shutdown lifecycle.
- `crates/island-core`: UI-independent, deterministic state machine, reducer, and transition policy.
- `crates/island-platform`: KDE Plasma 6 Wayland integration (window rules, overlap detection, geometry tracking).
- `crates/island-services`: Background asynchronous services (Clock, Battery, CAVA spectrum, MPRIS media, Notifications).
- `crates/island-ui`: Slint components, themes, animations, shell, and views.

## Building and Running

```bash
cargo build --release
cargo run -p island-app
```

## Running Tests

```bash
cargo test
```
