# KDE Dynamic Island

A premium, native Dynamic Island for **KDE Plasma 6 (Wayland)** built from the ground up in **Rust** and **Slint**.

## Features & Highlights

- **Pure Rust Architecture**: Clean, modular monolith with zero legacy C++/QML baggage.
- **Slint UI & Hardware Acceleration**: Sub-millisecond rendering with WGPU/FemtoVG.
- **KDE Plasma 6 Native Wayland Integration**:
  - Pinned directly to the top edge of the display with transparent borders and smooth concave corners.
  - Active window overlap detection via native `org_kde_plasma_window_management` Wayland protocol.
- **4 MVP Views**:
  1. **Compact View**: Minimalist pill with a procedural 3D Bloub pet, digital clock, and real-time CAVA audio visualizer / charging status.
  2. **Idle View**: Expanded dashboard with the 3D Bloub pet, clock, date, battery level, and a bottom bar. The avatar button opens a dedicated picker for shape, palette, eyes/expression, shuffle, and auto-random.
  3. **Idle Music View**: Seamless MPRIS media player integration displaying track title, artist, playback status, and media control buttons.
  4. **Welcome View**: Dynamic entrance greeting on system startup or unlock, automatically morphing to standard views after 3.5 seconds.

## Bloub Pet

The pet is a procedural WGPU body derived from Bloub, with 16 animated states, 16 Bloub
expressions plus a robot-eye variant, 8 shapes, and a configurable palette. Eyes blink more
frequently (sometimes twice), perform idle micro-expressions, and track the cursor; compact
mode uses larger eyes, faster blinks, and stronger gaze/acting. Clicking the pet cycles through
wink, bounce, backflip, and X-rotation stunts; desktop notifications, music, charging,
low-battery alerts, and dizzy states trigger their own animations. The avatar is static by default; the picker can shuffle
once or enable continuous random shape/palette/expression changes. It follows the global KWin
cursor through a small KWin script plus a D-Bus bridge; the script is installed and enabled
automatically on KDE.
If the WGPU backend is unavailable, the original vector pet is used as a fallback.

See `THIRD_PARTY_NOTICES.md` for Bloub attribution.

## Architecture

- `apps/island-app`: Bootstrap, wiring, runtime loop, and shutdown lifecycle.
- `crates/island-core`: UI-independent, deterministic state machine, reducer, and transition policy.
- `crates/island-platform`: KDE Plasma 6 Wayland integration (window rules, overlap detection, geometry tracking).
- `crates/island-services`: Background asynchronous services (Clock, Battery, CAVA spectrum, MPRIS media, Notifications, global cursor bridge).
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
