# Tasks — Render & Activity Policy

Track: R — Render activity (dikerjakan setelah Phase 0, sebelum Phase 1)  
PRD: [`../prd/10-render-policy.md`](../prd/10-render-policy.md)  
Index: [`../TASKS.md`](../TASKS.md)

---

## Trace

| Requirement | Task |
|---|---|
| `RENDER-1` | `TR1`, `TR2` |
| `RENDER-2` | `TR2`, `TR3`, `TR4` |
| `RENDER-3` | `TR4` |
| `RENDER-4` | `TR3` |
| `RENDER-5` | `TR4` |
| `RENDER-6` | `TR5` |
| `RENDER-7` | `TR2`, `TR5` |
| `RENDER-8` | `TR6` |

---

## TR1 — Tambah `RenderPolicy` murni

- **Status:** `[ ]`
- **Prioritas:** P0
- **Dependency:** `T0.4`
- **Trace:** `RENDER-1`
- **Lokasi:** `crates/island-core/src/render_policy.rs` (baru), `crates/island-core/src/lib.rs`.
- **Langkah:**
  1. Definisikan `RenderPolicy { main_visible, pet_3d, spectrum, animations }`.
  2. Implementasikan `render_policy(state: &AppState) -> RenderPolicy` sebagai fungsi murni.
  3. `main_visible` mengikuti visibility shell.
  4. `animations` false saat hidden.
  5. `pet_3d` hanya true jika tier aktif dan slot aktif benar-benar memakai pet.
  6. `spectrum` hanya true jika tier/slot aktif memakai visualizer dan media playing.
  7. Tambahkan test matrix Compact/Wide/Expand/hidden.
- **Acceptance:**
  - Policy dapat dites tanpa Slint/KDE.
  - Hidden selalu menghasilkan `pet_3d = false`, `spectrum = false`, `animations = false`.
  - Compact/Wide/Expand menghasilkan policy yang berbeda sesuai slot.
- **Risiko:** Sedang; jangan membuat aturan konten baru di policy. Policy hanya aktivitas render.

## TR2 — Wire `RenderPolicy` ke context dan runtime

- **Status:** `[ ]`
- **Prioritas:** P0
- **Dependency:** `TR1`
- **Trace:** `RENDER-1`, `RENDER-2`, `RENDER-7`
- **Lokasi:** `crates/island-ui/ui/context.slint`, `crates/island-ui/src/view_model.rs`, `apps/island-app/src/runtime.rs`.
- **Langkah:**
  1. Tambahkan properti read-only di `IslandContext`: `render-main-visible`, `render-pet`, `render-spectrum`, `render-animations`.
  2. Set properti dari `ViewModel` berdasarkan policy.
  3. Gunakan policy di runtime untuk memutuskan tick/redraw pet dan update spectrum.
  4. Pastikan input strip saat hidden hanya menangani reveal, bukan memicu render rutin.
  5. Jangan menghitung ulang policy di Slint.
- **Acceptance:**
  - Slint hanya membaca flag; tidak ada perhitungan policy di UI.
  - Runtime dapat membaca policy untuk pet, spectrum, dan timer.
  - Wiring tidak mengubah perilaku visual.
- **Risiko:** Rendah–sedang.

## TR3 — Gate pet 3D dan perbaiki `dirty`

- **Status:** `[ ]`
- **Prioritas:** P0
- **Dependency:** `TR2`
- **Trace:** `RENDER-2`, `RENDER-4`
- **Lokasi:** `apps/island-app/src/pet3d.rs`, `apps/island-app/src/runtime.rs`.
- **Langkah:**
  1. Tambahkan state active/visible pada `Pet3dInner` atau teruskan policy ke timer.
  2. `tick()` mengembalikan `dirty: false` tanpa sampling saat inactive.
  3. Timer utama berhenti memanggil `tick()`/`request_redraw()` saat hidden atau `render-pet = false`.
  4. Pastikan transisi hidden → visible melakukan satu tick dan redraw.
  5. Jangan mengubah FPS visible tanpa pengukuran.
- **Acceptance:**
  - Hidden: tidak ada tick 3D periodik dan tidak ada redraw periodik.
  - Pet yang tidak terlihat di slot aktif tidak di-tick.
  - Pet yang terlihat tetap ~30 FPS.
  - Test `hidden_does_not_tick` dan `visible_tick_is_dirty` lulus.
- **Risiko:** Sedang; jangan sampai pet terlihat tetapi tidak update.

## TR4 — Gate timer Slint dan aktivasi view

- **Status:** `[ ]`
- **Prioritas:** P1
- **Dependency:** `TR2`
- **Trace:** `RENDER-2`, `RENDER-3`, `RENDER-5`
- **Lokasi:** `crates/island-ui/ui/main.slint`, `components/pet-view.slint`, `components/blobatar-pet.slint`, `components/charging-indicator.slint`, `modules/**/*.slint`, `components/clock.slint` bila perlu.
- **Langkah:**
  1. Tambahkan properti `active` pada komponen berat (`PetView`, `BlobatarPet`, `ChargingIndicator`, prayer/alert slot).
  2. Gate `blink-timer`, `breath-timer`, `shake-timer`, `pulse-timer`, dan star timer dengan `active && render-animations`.
  3. Ganti `running: root.is-hovered || true` menjadi kondisi yang benar.
  4. Pastikan Compact hanya aktif saat `active-mode == 0`; Wide hanya saat `1`; Expand hanya saat `2`.
  5. Pertahankan crossfade 150–260 ms; jangan destroy view di tengah animasi.
- **Acceptance:**
  - Tidak ada timer `|| true`.
  - Timer tier nonaktif berhenti.
  - Crossfade tetap terlihat.
  - Tidak ada regresi visual pada tiga tier.
- **Risiko:** Tinggi; test per tier dan per state media/charging/prayer.

## TR5 — Gate spectrum dan update UI hidden

- **Status:** `[ ]`
- **Prioritas:** P1
- **Dependency:** `TR2`
- **Trace:** `RENDER-6`, `RENDER-7`
- **Lokasi:** `crates/island-ui/src/view_model.rs`, `apps/island-app/src/runtime.rs`, `components/cava-visualizer.slint`, `modules/media/*.slint`.
- **Langkah:**
  1. Jangan menulis `b0..b7` ke context saat `render-spectrum = false`.
  2. Jangan memicu update visualizer di slot nonaktif.
  3. Tetapkan nilai nol satu kali saat spectrum berhenti, bukan setiap frame.
  4. CAVA service boleh tetap berjalan; lifecycle-nya tidak diubah di task ini.
  5. Jangan melakukan full UI sync untuk spectrum-only change; detail diff ada di `T7.3`.
- **Acceptance:**
  - Hidden tidak memperbarui spectrum UI.
  - Compact/Wide non-media tidak memperbarui spectrum.
  - Media playing di Compact/Expand tetap menampilkan spectrum.
- **Risiko:** Sedang.

## TR6 — Instrumentasi dan pengukuran render

- **Status:** `[ ]`
- **Prioritas:** P2
- **Dependency:** `TR3`, `TR4`, `TR5`
- **Trace:** `RENDER-8`
- **Lokasi:** `apps/island-app/src/pet3d.rs`, `apps/island-app/src/runtime.rs`, `crates/island-ui/src/view_model.rs`.
- **Langkah:**
  1. Tambahkan counter: pet tick/detik, request redraw/detik, UI sync/detik.
  2. Log counter secara periodik pada level `debug` atau `trace`.
  3. Ukur baseline hidden, Compact idle, Compact playing, Wide, Expand.
  4. Dokumentasikan hasil sebelum/sesudah di task atau PRD.
- **Acceptance:**
  - Ada angka baseline dan sesudah.
  - Hidden menunjukkan tick/redraw mendekati nol.
  - Tidak ada logging berlebihan di level `info`.
- **Risiko:** Rendah.
