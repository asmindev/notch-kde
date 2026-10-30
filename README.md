# notchde — KDE Dynamic Island

A premium, native **Dynamic Island** for **KDE Plasma 6 (Wayland)** built from the ground up in **Rust** and **Slint**.

---

## Highlights & Features

- **Pure Rust Architecture**: Clean, modular monolith with zero legacy C++ or QML dependencies.
- **Hardware-Accelerated Slint UI**: Sub-millisecond rendering with WGPU/FemtoVG and ultra-low CPU/memory usage.
- **Native KDE Plasma 6 Wayland Integration**:
  - Anchored directly to the top edge of the display with smooth concave corners and custom drop shadows.
  - Active window overlap detection via native `org_kde_plasma_window_management` Wayland protocol (auto-hides when maximized/tiled windows touch the top bar; smoothly reveals on hover).
- **Strict Energy & Render Policy**:
  - Zero-waste rendering engine (`RenderPolicy`): spectrum visualizers, 3D pet updates, and UI animation timers are completely suspended when hidden, docked, or paused.

---

## Views & Tiers

`notchde` features a multi-tiered adaptive presentation system:

### 1. Compact View (Pill Notch)
- **Standard Mode**: Displays the animated Bloub pet, real-time digital clock, battery status/charging indicator, and CAVA audio visualizer bars.
- **Eyes-Only Mode (*Hanya Mata*)**: A clean, expressive pair of Bloub eyes centered on the notch.
- **CAVA Ambient Aura (*Aura CAVA Musik*)**: A soft, mirrored Gaussian-blurred soundscape bloom rendered behind the eyes in Eyes-Only mode (adjustable from 0% to 100% opacity in Settings).

### 2. Wide View (Quick Status & Media)
- **Media Player**: Live MPRIS player integration featuring track title, artist, playback controls, album art, interactive audio spectrum, and a compact app-source selector with icons (Spotify, Firefox, Chrome, VLC, Cider, Amberol, etc.).
- **Prayer Alert & Schedule**: Upcoming prayer information, time countdown, and city indicators.
- **System Dashboard**: Bloub pet avatar, digital clock, date, and battery level.
- **Welcome Entrance**: Smooth greeting animation on system startup or screen unlock.

### 3. Expand View (Full Interactive Dashboard)
- **Expanded Media**: Detailed media controls, seekbar, and full audio spectrum.
- **Astronomical Prayer Dome (*Kubah Sholat*)**:
  - Visual daytime and nighttime arcs tracking real-time solar and lunar orbits.
  - **Dynamic Day/Night Horizon Timeline**:
    - **Night Phase (*Malam*)**: Maghrib (left/sunset) → Isya (early night) → Subuh (right/dawn) following the moon's trajectory.
    - **Day Phase (*Siang*)**: Subuh (left/dawn) → Dzuhur (zenith/noon) → Ashar (afternoon) → Maghrib (right/sunset) following the sun's trajectory.
  - Real-time sky twilight and astral night gradient calculation with star field opacity.

---

## Bloub Pet (Procedural Mascot)

The interactive desktop companion is a procedural 3D WGPU body derived from **Bloub**:
- **16 Animated States**: Idle acting, playful blinking, music dancing, dizzy spinning (from rapid poking), sleepy yawning (on low battery), and charging excitement.
- **Interactive Stunts**: Clicking the pet cycles through playful animations (wink, bounce, backflip, and X-axis rotation).
- **Cursor Gaze Tracking**: Pet smoothly follows your mouse cursor across the screen.
- **Customizable Avatar**: 8 morphing 3D shapes (Cercle, Galet, Squircle, Capsule, Triangle, Hexagone, Nuage, Goutte), 16 expressions (plus robot-eyes), and 14 color palettes.
- **2D CPU Fallback**: Automatically falls back to a clean flat vector renderer if hardware WGPU acceleration is unavailable.

---

## Settings Window

An intuitive multi-tab configuration interface:
- **Avatar**: Choose shape, palette, and expressions; toggle auto-randomization or shuffle on demand.
- **Perilaku (Behavior)**: Enable/disable cursor gaze, idle acting, click stunts, notification reactions, music reactions, and sleepy low-battery behavior.
- **Tampilan (Appearance & System)**:
  - Toggle between Standard and Eyes-Only compact styles.
  - Adjust CAVA background aura intensity slider (0%–100%).
  - Configure floating island drop shadow (intensity, radius, offset X/Y).
  - Select media visual focus (Artwork-primary vs. Pet-primary).
  - Choose between 3D (WGPU) and 2D (Flat CPU) mascot renderers.
- **Sholat (Prayer Schedule)**: Configure Indonesian city presets (Jakarta, Kendari, Bau-Bau, etc.), reminder windows, and adhan alerts.
- **Tentang (About)**: Version information and open-source acknowledgements.

Settings are saved atomically to `$XDG_CONFIG_HOME/notchde/settings.json` (falling back to `~/.config/notchde/settings.json`).

---

## Architecture

`notchde` is structured as a high-performance modular monolith across 6 crates:

```text
apps/island-app          Entry point, runtime loop, bootstrap, settings store
crates/island-core       Deterministic state machine, reducer, view engine, prayer domain, render policy
crates/island-pet3d      Procedural 3D WGPU renderer & 2D flat mascot fallback
crates/island-platform   KDE Plasma 6 Wayland window integration, overlap detection, geometry
crates/island-services   Async background workers (Clock, Battery, CAVA, MPRIS media, Notifications)
crates/island-ui         Slint UI components, view models, themes, shell, and animations
```

---

## Building & Running

### Prerequisites

- **Linux** with **KDE Plasma 6 (Wayland session)**
- **Rust toolchain** (Edition 2024, Rust 1.85+)
- **CAVA** (`cava`) installed and available in `$PATH` (for audio spectrum visualizer)
- Development libraries for Wayland and Vulkan/OpenGL

### Build & Run

```bash
# Clone the repository
git clone https://github.com/asmindev/notchde.git
cd notchde

# Build in release mode
cargo build --release

# Run island-app
cargo run -p island-app
```

---

## Testing

The repository maintains strict test coverage with 226 unit and integration tests across all workspace crates:

```bash
# Run all workspace tests
cargo test --workspace --tests

# Check formatting
cargo fmt --all -- --check

# Run linter
cargo clippy --workspace --all-targets
```

---

## Credits & Acknowledgements

This project is made possible thanks to the following open-source projects and authors:

- **[Bloub](https://github.com/jeremy-prt/bloub)** by **Jérémy Perret** — The original procedural 3D SVG bot avatar and expressions ported to Rust/WGPU (licensed under MIT).
- **[Slint UI](https://slint.dev/)** — The next-generation declarative GUI framework providing smooth, memory-efficient, hardware-accelerated rendering.
- **[KDE Community](https://kde.org/)** — For KDE Plasma 6 and KWin Wayland protocols enabling seamless desktop window management.
- **[CAVA](https://github.com/karlstav/cava)** by **Karl Stavestrand** — Console-based Audio Visualizer for real-time audio spectrum capture via PulseAudio/PipeWire.
- **Dynamic Island Concept** — Originally designed by Apple, reimagined and extended natively for the Linux Wayland desktop.
- **Kemenag RI / Astronomical Calculations** — Reference formulas and solar algorithms for accurate Indonesian prayer times and atmospheric twilight gradients.

---

## License

- This project is licensed under the **GNU General Public License v3.0 (GPL-3.0-or-later)**. See [`LICENSE`](LICENSE) for details.
- Third-party components and ports are covered by their respective licenses (see [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)).
