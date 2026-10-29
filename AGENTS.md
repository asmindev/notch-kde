# AGENTS.md

Panduan kerja untuk agent/developer di repository notchde.

---

## 1. Baca Dulu

Sebelum mengubah kode:

1. Baca [`docs/PRD.md`](docs/PRD.md) untuk tujuan dan batas arsitektur.
2. Baca [`docs/TASKS.md`](docs/TASKS.md) untuk memilih task dan dependency.
3. Baca domain PRD/task yang relevan. Untuk perubahan view, animasi, timer UI, CAVA, atau pet, baca juga [`docs/prd/10-render-policy.md`](docs/prd/10-render-policy.md).
4. Baca [`docs/TRACEABILITY.md`](docs/TRACEABILITY.md) untuk memastikan requirement yang dicakup.
5. Baca kode di sekitar area yang akan diubah, termasuk test terdekat.

Dokumen PRD adalah target arsitektur. Kode adalah sumber kebenaran perilaku saat ini sampai task terkait selesai.

---

## 2. Ringkasan Project

- **Produk:** Dynamic Island untuk KDE Plasma 6 Wayland.
- **Stack:** Rust edition 2024, Slint 1.18, Tokio, zbus, Wayland.
- **Bentuk:** Modular monolith, 6 workspace member.
- **Baseline test:** `cargo test --workspace` lulus 133 test.
- **Render policy:** Aktivitas render/pet/spectrum/timer ditentukan oleh `RenderPolicy`; hidden tidak boleh melakukan render kontinu.

### Struktur

```text
apps/island-app          entry point, bootstrap, runtime, settings store
crates/island-core       state, reducer, view engine, settings, prayer domain
crates/island-ui         ViewModel, content router, Slint UI
crates/island-platform   KDE/Wayland window, overlap, geometry
crates/island-services   clock, media, battery, dnd, audio, cava, prayer, dll
crates/island-pet3d      renderer/animasi pet
docs/                    PRD, task, traceability
packaging/               packaging desktop
```

---

## 3. Command Penting

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo build --workspace
cargo run -p island-app
```

Catatan:

- `cargo run -p island-app` membutuhkan session KDE Plasma 6 Wayland untuk perilaku penuh.
- Root `tests/` sudah dihapus di `T0.1`. Test baru harus berada di `crates/<crate>/tests/` atau unit test dekat kode.
- Jangan menambah dependency baru tanpa alasan kuat; cek `Cargo.toml` workspace terlebih dahulu.
- Verifikasi render hidden/timer/pet membutuhkan runtime KDE; test policy murni tidak boleh bergantung pada session.

---

## 4. Aturan Arsitektur

### 4.1 Dependency direction

```text
island-app -> island-ui -> island-core
           -> island-services -> island-core
           -> island-platform -> island-core
           -> island-pet3d
```

- `island-core` tidak boleh bergantung pada Slint, zbus, Wayland, filesystem, atau environment.
- `island-ui` tidak boleh mengambil alih logika domain/service.
- `island-services` tidak boleh mengetahui UI, view, module, atau `AppState`.
- `island-platform` menangani OS/window/overlap; tidak berisi aturan konten.
- `island-app` menjadi orchestrator/wiring, bukan tempat aturan bisnis baru.

### 4.2 Module

- Module hanya menerima `ModuleContext` read-only.
- Module tidak boleh mengakses service, filesystem, UI, atau channel.
- Registrasi module melalui `ModuleRegistry`; jangan menambah module langsung di `ViewEngine`.
- UI mapping module melalui registry UI, bukan angka `active-module` yang tersebar.
- Jangan membuat dynamic plugin/DI container.

### 4.3 Content

- `ContentResolver` adalah satu-satunya penentu module aktif.
- Slint tidak boleh memiliki aturan `show-system`/`show-media`/`show-prayer`.
- View hanya merender hasil resolver.
- Override prayer panel/source selector harus eksplisit dan punya `ContentReason`.

### 4.4 State

- `ViewEngine` adalah pemilik canonical tier/hover/overlap/visibility.
- Jangan menambah field navigasi duplikat di `AppState`.
- Jangan menulis field navigasi langsung dari reducer; gunakan API engine.
- UI menerima snapshot read-only, bukan `AppState` penuh.

### 4.5 Service

- Service mengikuti kontrak `Service`/`ServiceHandle` setelah Phase 5.
- Service hanya mengirim payload domain atau `DomainEvent`; bukan aksi UI.
- Jangan menambah global mutable baru.
- Setiap child process wajib memiliki jalur shutdown.

### 4.6 Timer

- Gunakan `TimerId` bertipe setelah Phase 4.
- Jangan menambah timer string baru.
- Scheduling ulang ID yang sama harus membatalkan timer lama.
- Timer mengirim event; timer tidak mengubah state domain secara langsung.

### 4.7 Konfigurasi

- `AppConfig` untuk deployment/internal; `PersistedSettings` untuk user.
- Jangan membaca environment di `island-core`.
- Jangan panic pada config/platform yang hilang; gunakan fallback + warning.
- Dimensi window/notch/input region harus berasal dari `WindowSpec`, bukan angka tersebar.

### 4.8 Render & activity

- Aktivitas render, pet 3D, spectrum, dan animasi ditentukan oleh `RenderPolicy` murni.
- Hidden berarti tidak ada redraw kontinu, tick pet, timer animasi, atau update spectrum UI.
- Di luar crossfade, hanya view tier aktif yang boleh dilukis.
- Jangan destroy view di tengah transisi; pertahankan wrapper dan gate heavy children.
- Jangan menulis `running: true` atau `running: ... || true` pada timer UI.
- Timer animasi harus digate oleh visibility/tier aktif; timer service mengikuti lifecycle service.
- Jangan menghitung ulang policy di Slint; Slint hanya membaca flag read-only.
- `RenderPolicy` tidak boleh mengandung aturan konten; itu tanggung jawab `ContentResolver`.
- Jangan mengubah FPS atau durasi animasi tanpa pengukuran dan persetujuan task.

---

## 5. Alur Kerja Task

1. Pilih task dari `docs/TASKS.md` sesuai dependency.
2. Baca requirement PRD dan bagian `Trace` pada file task.
3. Ubah status task menjadi `[~]` sebelum mulai.
4. Implementasikan perubahan minimal yang memenuhi task.
5. Tambahkan test untuk logika yang berubah.
6. Jalankan `cargo fmt`, `cargo clippy`, dan `cargo test`.
7. Perbarui status task menjadi `[x]` jika semua acceptance terpenuhi.
8. Perbarui PRD/traceability jika ada keputusan atau requirement baru.
9. Jangan commit kecuali diminta secara eksplisit.

Aturan tambahan:

- Track `TR1`–`TR6` (render activity) dikerjakan setelah Phase 0 dan sebelum Phase 1.
- Satu task = satu perubahan yang dapat direview.
- Jangan mencampur refactor besar dengan fitur baru.
- Jangan memperbaiki hal di luar task tanpa mencatatnya sebagai task baru.
- Jika menemukan bug yang memblokir, catat di task dan minta keputusan bila perlu.

---

## 6. Testing

- Unit test dekat kode yang diuji; integration test di `crates/<crate>/tests/`.
- Prioritaskan test murni untuk reducer, resolver, registry, timer, render policy, dan config.
- Characterization test harus mendokumentasikan perilaku aktual, termasuk bug yang diketahui.
- Test render policy harus mencakup matrix Compact/Wide/Expand/hidden.
- Jangan bergantung pada session KDE untuk test logika core.
- Test service/child process harus memastikan shutdown.
- Jika mengubah perilaku yang sudah dites, perbarui test dan jelaskan di task.

Perintah verifikasi minimum:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

---

## 7. Gaya Kode

- Ikuti gaya dan pola file sekitar.
- Gunakan nama domain yang sudah ada (`AppEvent`, `Effect`, `ViewEngine`, `ModuleContext`, dll).
- Hindari komentar yang mengulang kode; komentar hanya untuk alasan/desain yang tidak jelas.
- Jangan menambah `unwrap`/`expect` pada jalur startup/config/platform yang dapat gagal karena lingkungan.
- Gunakan `tracing` dengan konteks.
- Jangan menambah emoji ke kode/dokumen kecuali diminta.
- Jangan meninggalkan kode mati atau parameter unused.

---

## 8. Pitfall yang Diketahui

| Pitfall | Lokasi | Aturan |
|---|---|---|
| Test di root repo | `tests/` | Jangan buat folder root; gunakan test crate |
| State navigasi ganda | `state.rs`, `sync_engine()` | Jangan tambah salinan state baru |
| Aturan konten di Slint | `views/*.slint` | Pindahkan ke resolver |
| Timer string | `events.rs`, `runtime.rs` | Gunakan `TimerId` |
| Geometry tersebar | `runtime.rs`, `input_region.rs`, `bootstrap.rs`, `metrics.slint` | Gunakan `WindowSpec` |
| Env di core | `transition_policy.rs` | Jangan baca env dari core |
| Reload album art | `view_model.rs` | Cache berdasarkan `art_url` |
| Full UI sync saat spectrum | `view_model.rs`, `runtime.rs` | Diff per bagian; gate spectrum |
| Pet 30 FPS saat hidden | `pet3d.rs` | Gate tick/redraw via `RenderPolicy` |
| Timer UI `running: true` / `\|\| true` | `*.slint` | Gate dengan active/visibility |
| View nonaktif ikut dilukis | `main.slint`, `views/*.slint` | Hanya tier aktif di luar crossfade |
| Global media | `services/media.rs` | Gunakan instance |
| Panic platform | `kde/mod.rs` | Kembalikan `Result`/fallback |
| API mati | `view_engine.rs`, `transition_policy.rs` | Hapus/rekonsiliasi, jangan pakai |

---

## 9. Batasan

- Jangan melakukan rewrite total.
- Jangan menambah dynamic plugin, DI container, atau crate runtime baru tanpa persetujuan.
- Jangan mengubah perilaku visual yang tidak diminta.
- Jangan menambah render loop atau timer UI baru tanpa gate visibility.
- Jangan commit, push, atau membuat PR kecuali diminta.
- Jangan menyentuh file yang tidak terkait task.
