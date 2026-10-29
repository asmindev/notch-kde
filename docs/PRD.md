# PRD — Modularisasi Arsitektur notchde

Status: Draft  
Tanggal: 2026-10-29  
Basis: audit source code aktual, bukan asumsi desain  
Baseline verifikasi: `cargo test --workspace` lulus 187 test

---

## 1. Cara Membaca Dokumen

PRD ini dipecah per domain agar requirement, keputusan, dan task dapat ditelusuri tanpa membaca satu dokumen besar.

| Domain | Requirement | PRD | Task |
|---|---|---|---|
| Baseline & kualitas | `BASE-*`, `TEST-*`, `ERR-*` | [`prd/08-quality.md`](prd/08-quality.md) | [`tasks/00-baseline.md`](tasks/00-baseline.md) |
| Module system | `MOD-*` | [`prd/01-module-system.md`](prd/01-module-system.md) | [`tasks/01-module-system.md`](tasks/01-module-system.md) |
| Content resolution | `CONTENT-*` | [`prd/02-content-resolution.md`](prd/02-content-resolution.md) | [`tasks/02-content-resolution.md`](tasks/02-content-resolution.md) |
| View & state | `VIEW-*`, `STATE-*` | [`prd/03-view-state.md`](prd/03-view-state.md) | [`tasks/03-view-state.md`](tasks/03-view-state.md) |
| Timer & effect | `TIMER-*` | [`prd/04-timers-effects.md`](prd/04-timers-effects.md) | [`tasks/04-timers-effects.md`](tasks/04-timers-effects.md) |
| Services | `SVC-*` | [`prd/05-services.md`](prd/05-services.md) | [`tasks/05-services.md`](tasks/05-services.md) |
| Configuration | `CFG-*` | [`prd/06-configuration.md`](prd/06-configuration.md) | [`tasks/06-configuration.md`](tasks/06-configuration.md) |
| Performance | `PERF-*` | [`prd/07-performance.md`](prd/07-performance.md) | [`tasks/07-performance.md`](tasks/07-performance.md) |
| Render & activity | `RENDER-*` | [`prd/10-render-policy.md`](prd/10-render-policy.md) | [`tasks/09-render-policy.md`](tasks/09-render-policy.md) |
| Optional cleanup | `OPT-*` | [`prd/09-optional-cleanup.md`](prd/09-optional-cleanup.md) | [`tasks/08-optional-cleanup.md`](tasks/08-optional-cleanup.md) |

Peta requirement ke task ada di [`TRACEABILITY.md`](TRACEABILITY.md).  
Aturan kerja agent/developer ada di [`../AGENTS.md`](../AGENTS.md).

---

## 2. Ringkasan

notchde adalah Dynamic Island untuk KDE Plasma 6 Wayland yang dibangun dengan Rust dan Slint. Aplikasi sudah berjalan sebagai modular monolith dengan pemisahan crate yang baik, tetapi batas tanggung jawab antara module, service, engine, view, context, dan konfigurasi belum konsisten.

PRD ini menetapkan target arsitektur dan kriteria penerimaan untuk perbaikan inkremental. Fokusnya bukan rewrite, melainkan membuat penambahan module, service, dan view dapat dilakukan tanpa menyentuh banyak bagian core dan tanpa menduplikasi aturan konten.

---

## 3. Latar Belakang dan Temuan Utama

1. **Module baru hanya metadata.** `IslandModule` hanya menyediakan `id`, `name`, `tiers`, dan `priority`. UI memilih konten lewat angka `active-module` yang di-hardcode. Menambah module menyentuh enum core, implementasi module, registrasi engine, mapping `view_model.rs`, dan kondisi di tiga view Slint.
2. **Aturan pemilihan konten terduplikasi.** Priority di `island-core`, override langsung di reducer, dan kondisi `show-system`/`show-media`/`show-prayer` di Slint memiliki semantik yang tidak selalu sama. Contoh: media paused bisa dipilih sebagai module Media oleh engine, tetapi WideView merender System.
3. **`AppState` adalah god object dengan dual ownership.** Field navigasi disalin dari `ViewEngine` melalui `sync_engine()`. Beberapa reducer menulis langsung ke engine. Ini rawan divergensi.
4. **Runtime dan service lifecycle (selesai Phase 5).** Service memiliki kontrak `Service`/`ServiceHandle`, health state, sinyal shutdown, dan supervisor dengan timeout. Sisa kompleksitas callback UI tetap menjadi batas runtime/UI.
5. **Timer typed dan executor (selesai Phase 4).** `Effect::ScheduleTimer` memakai `TimerId`; `TimerScheduler` menyimpan handle per ID dan `EffectExecutor` memisahkan eksekusi effect dari event loop.
6. **Sinkronisasi UI penuh setiap perubahan.** Album art dimuat ulang dari disk dan jadwal prayer dihitung ulang pada setiap update, termasuk saat spectrum CAVA berjalan 30 FPS.
7. **Konfigurasi tersebar.** Dimensi window di-hardcode di beberapa file, `ISLAND_DODGE` dibaca di `island-core`, dan path config tersebar.
8. **API mati dan test orphan.** `handle_media_update`, `resolve_view_on_*`, dan root `tests/` yang tidak termasuk workspace dapat menyesatkan. Root `tests/` sudah dihapus di `T0.1`; API mati sudah dihapus di `T0.2`.
9. **Render dan timer tidak mengikuti visibility.** Tiga view selalu dibuat, pet 3D berjalan ~30 FPS tanpa cek hidden, dan beberapa timer Slint berjalan tanpa syarat meskipun komponen tidak terlihat.

---

## 4. Tujuan

### 4.1 Tujuan produk

- Menambah module baru tanpa mengubah `ViewEngine`, platform, atau shell utama.
- Menambah service baru tanpa mengubah event loop UI secara besar.
- Memiliki satu sumber kebenaran untuk keputusan konten yang tampil.
- Memisahkan konfigurasi deployment/internal dari preferensi pengguna.
- Menjaga perilaku dan tampilan existing sebagai baseline.
- Menurunkan biaya update UI yang tidak perlu.
- Menghentikan render, animasi, dan spectrum saat konten tidak terlihat.

### 4.2 Tujuan teknis

- Module direpresentasikan oleh deskriptor dan registry.
- Pemilihan konten dilakukan oleh `ContentResolver`, bukan oleh Slint.
- `ViewEngine` hanya mengurus tier view, hover, overlap, dan visibility.
- State navigasi memiliki satu pemilik canonical.
- Service memiliki lifecycle, health, dan shutdown yang seragam.
- Timer memakai tipe terstruktur dan dapat dibatalkan.
- Konfigurasi memiliki skema, validasi, dan fallback tanpa panic.
- Render activity ditentukan oleh `RenderPolicy` murni, bukan kondisi tersebar di Slint.
- Test mencakup reducer, resolver, registry, config, timer, render policy, dan lifecycle service.

---

## 5. Non-Goals

- Dynamic plugin loading atau ABI module eksternal.
- Framework dependency injection.
- Memecah aplikasi menjadi microservice atau menambah crate runtime baru sebelum diperlukan.
- Rewrite total Slint UI.
- Mengganti arsitektur reducer/event yang sudah ada hanya demi pola tertentu.
- Menambah fitur produk baru yang tidak berkaitan dengan modularisasi.
- Automated visual regression testing tanpa infrastruktur yang jelas.

---

## 6. Pengguna dan Use Case

### 6.1 Pengguna

1. **Pengguna desktop KDE** — menginginkan island yang stabil, ringan, dan responsif.
2. **Developer module** — ingin menambah konten baru tanpa memahami seluruh internal aplikasi.
3. **Maintainer** — ingin menelusuri aliran state, service, dan konfigurasi dengan cepat.

### 6.2 Use case utama

- Menambah module Timer/Fokus dengan slot Compact, Wide, dan Expand.
- Menambahkan service baru, misalnya kalender atau cuaca, tanpa mengubah view.
- Mengubah aturan module mana yang tampil saat media paused.
- Mengubah ukuran window tanpa mismatch antara tampilan dan input region.
- Menonaktifkan dodge/overlap lewat konfigurasi, bukan environment variable di core.
- Melakukan shutdown bersih saat sesi desktop berakhir.

---

## 7. Prioritas Domain

| Prioritas | Domain | Alasan |
|---|---|---|
| P0 | Baseline & kualitas | Mencegah baseline yang menyesatkan |
| P0 | Render & activity | Menghentikan pekerjaan background yang boros |
| P0 | Module system | Fondasi ekstensibilitas |
| P0 | Content resolution | Menghilangkan duplikasi aturan konten |
| P1 | View & state | Mencegah divergensi state |
| P1 | Timer & effect | Mengurangi bug timer dan duplikasi |
| P1 | Services | Stabilitas dan testability |
| P1 | Configuration | Menghilangkan config tersebar dan panic |
| P2 | Performance | Perbaikan performa nyata |
| P2 | Optional cleanup | Hanya jika kompleksitas mulai naik |

---

## 8. Target Arsitektur

```text
UI (Slint)
  actions -> RuntimeOrchestrator
               -> Reducer -> AppState
               -> EffectExecutor -> Services / Timers / Settings
               -> ContentResolver + ModuleRegistry
               -> RenderPolicy -> Pet3d / Timers / Spectrum
               -> UiSyncEngine -> ViewSnapshot -> Slint

Services -> DomainEvent -> Runtime -> Reducer
Platform -> Overlap/Geometry event -> Runtime -> Reducer
```

### 8.1 Batas tanggung jawab

| Komponen | Tanggung jawab |
|---|---|
| `ViewEngine` | Tier view, hover, overlap, reveal/dismiss, visibility |
| `ContentResolver` | Memilih module konten dari snapshot dan registry |
| `ModuleRegistry` | Registrasi dan lookup module |
| `EffectExecutor` | Menjalankan efek domain |
| `TimerScheduler` | Timer typed, generation, cancel |
| `ServiceSupervisor` | Start/stop/health service |
| `RenderPolicy` | Menentukan aktivitas render, pet, spectrum, animasi |
| `UiSyncEngine` | Diff snapshot ke context, caching |
| `Pet3dEngine` | Animasi/render pet |

### 8.2 Prinsip integrasi

- `island-core` tidak boleh bergantung pada Slint, zbus, filesystem, atau environment.
- Module hanya menerima `ModuleContext` read-only.
- Service hanya mengirim payload domain atau `DomainEvent`; tidak mengirim aksi UI.
- Slint tidak boleh memiliki aturan bisnis konten.
- Slint tidak menentukan sendiri kapan animasi/render harus berjalan; gunakan `RenderPolicy`.
- Konfigurasi deployment masuk melalui `AppConfig`; preferensi user melalui `PersistedSettings`.
- Setiap penambahan module/service harus dapat dites tanpa menjalankan session KDE penuh.

---

## 9. Non-Functional Requirements

| Area | Target |
|---|---|
| Kompatibilitas | KDE Plasma 6 Wayland; perilaku existing tidak berubah |
| Bahasa | Rust edition 2024, Slint 1.18 |
| Build | `cargo build --workspace` dan `cargo test --workspace` tetap lulus |
| Lint | `cargo fmt --all -- --check` dan `cargo clippy --workspace --all-targets` bersih atau tidak menambah warning baru |
| Startup | Tidak ada panic jika tool KDE/config tidak tersedia |
| Shutdown | Service berhenti dalam batas timeout yang ditentukan |
| Memory | Tidak ada cache tak terbatas untuk album art/model |
| Render | Hidden tidak menjalankan redraw kontinu, tick pet, timer animasi, atau spectrum UI |
| Observability | Setiap service memiliki status dan log kegagalan; render activity dapat diukur |
| Maintainability | Penambahan module tidak menyentuh `ViewEngine` dan shell |

---

## 10. Metrik Keberhasilan

| Metrik | Baseline | Target |
|---|---|---|
| Jumlah titik perubahan untuk module baru | 5+ area | ≤ 3 area (module core, slot UI, registry UI) |
| Duplikasi aturan konten | Rust + reducer + 3 Slint view | Hanya resolver |
| Service dengan lifecycle seragam | 9 dari 9 | Semua |
| Timer string ID | 13+ cabang | 0 |
| Test workspace | 187 lulus | ≥ 187 dan bertambah untuk performance |
| Reload album art saat spectrum | Berpotensi setiap update | 0 kali jika `art_url` sama |
| Tick pet 3D saat hidden | ~30 FPS | 0 |
| Timer animasi saat hidden | Beberapa timer aktif | 0 timer non-input |
| View nonaktif yang dilukis | Compact/Wide/Expand selalu dibuat | Hanya tier aktif di luar crossfade |
| Panic startup karena tool KDE | Mungkin | Tidak ada |

---

## 11. Milestone

| Milestone | Isi | Gate |
|---|---|---|
| M0 — Baseline | Test orphan, API mati, dependency bersih | `cargo test --workspace` lulus; tidak ada test root orphan |
| M0.5 — Render Activity | RenderPolicy, gate pet/timer/spectrum, pengukuran | Hidden tanpa redraw/tick periodik; tidak ada regresi visual |
| M1 — Module System | ModuleId, descriptor, registry, UI binding | Module dummy dapat ditambahkan tanpa ViewEngine |
| M2 — Content Resolution | Resolver, override, hapus aturan konten Slint | Test matrix content lulus |
| M3 — Runtime Stability | State canonical, timer typed, service lifecycle | Shutdown bersih dan test timer/service lulus |
| M4 — Configuration | AppConfig, WindowSpec, validasi, migration | Tidak ada env di core; tidak ada panic config/platform |
| M5 — Performance | Cache art/prayer/model, timer gating | Smoke test menunjukkan tidak ada reload berulang |

---

## 12. Risiko Lintas Domain

| Risiko | Dampak | Mitigasi |
|---|---|---|
| Resolver mengubah perilaku media paused | UI terasa berubah | Characterization test sebelum refactor; bandingkan snapshot lama/baru |
| Service supervisor menambah kompleksitas | Runtime lebih sulit dibaca | Kontrak minimal; migrasi satu service per commit |
| Config split memecahkan settings lama | Preferensi hilang | Migration test dan default backward-compatible |
| Timer typed mengubah timing | Animasi overlap berubah | Pertahankan durasi dan generation; test sequencing |
| UI sync diff salah invalidation | Tampilan stale | Cache key eksplisit; test snapshot per bagian |
| Render gating terlalu agresif | Animasi terpotong atau pet membeku | Pisahkan flag `main_visible`/`animations`; test matrix tier |
| Hidden input strip tetap memicu event | Event rate tidak turun | Ukur event rate; dokumentasikan tradeoff reveal |
| Refactor menyentuh banyak file | Konflik dan regresi | Phase kecil, tidak mencampur beberapa phase dalam satu perubahan |
| Root tests menyesatkan | Bug tidak terdeteksi | Hapus/pindahkan di Phase 0 |

---

## 13. Asumsi dan Pertanyaan Terbuka

### Asumsi

- Jumlah module tetap kecil dan dikompilasi bersama aplikasi; dynamic plugin tidak dibutuhkan.
- Slint tetap menjadi UI framework dan view utama tidak ditulis ulang.
- KDE Plasma 6 Wayland tetap target platform utama.
- Tidak ada kebutuhan multi-instance service dalam waktu dekat.

### Pertanyaan terbuka

1. Apakah `is_welcome` akan tetap menjadi state presentasi atau dipromosikan menjadi module/overlay `Welcome`? Lihat [`prd/09-optional-cleanup.md`](prd/09-optional-cleanup.md).
2. Apakah window settings membutuhkan context terpisah penuh, atau cukup namespace properti? Lihat `OPT-1`.
3. Apakah service health perlu tampil di UI Settings/About, atau cukup di log? Lihat `ERR-3`.
4. Apakah config aplikasi perlu file `config.json`, atau cukup default + env developer? Lihat `CFG-1` dan `CFG-2`.
5. Apakah CAVA tetap memakai config sendiri atau dipindahkan ke `AppConfig`? Lihat `CFG-1`.
6. Apakah surface/input strip perlu dipertahankan saat hidden untuk reveal, atau window harus benar-benar di-hide? Lihat `RENDER-7`.

### Keputusan render

- Hanya view tier aktif yang dilukis setelah crossfade; transisi boleh melukis dua view.
- Hidden berarti tidak ada render kontinu, tick pet, timer animasi, atau update spectrum UI.
- View wrapper dipertahankan; heavy children yang di-gate, bukan destroy seluruh view.
- Input strip boleh tetap ada untuk reveal, tetapi tidak boleh memicu render work.

---

## 14. Referensi Source Code

- Entry/runtime: `apps/island-app/src/main.rs`, `bootstrap.rs`, `runtime.rs`
- Core state/reducer: `crates/island-core/src/state.rs`, `reducer.rs`
- View engine: `crates/island-core/src/view_engine.rs`, `transition_policy.rs`
- Module UI: `crates/island-ui/ui/views/*.slint`, `ui/modules/**`
- Render activity: `crates/island-ui/ui/main.slint`, `ui/shell/island-shell.slint`, `apps/island-app/src/pet3d.rs`, `apps/island-app/src/input_region.rs`
- UI sync: `crates/island-ui/src/view_model.rs`, `content_router.rs`
- Services: `crates/island-services/src/*.rs`
- Platform: `crates/island-platform/src/kde/mod.rs`, `overlap.rs`, `window.rs`
- Config: `apps/island-app/src/settings_store.rs`, `crates/island-core/src/settings.rs`
- Test baseline: `crates/island-core/tests/**`, `crates/*/src/**/tests`; root `tests/**` sudah dihapus di `T0.1`
