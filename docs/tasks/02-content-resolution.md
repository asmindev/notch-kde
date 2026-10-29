# Tasks — Content Resolution

Fase: 2  
PRD: [`../prd/02-content-resolution.md`](../prd/02-content-resolution.md)  
Index: [`../TASKS.md`](../TASKS.md)

---

## Trace

| Requirement | Task |
|---|---|
| `CONTENT-1` | `T2.2`, `T2.4` |
| `CONTENT-2` | `T2.2` |
| `CONTENT-3` | `T2.2` |
| `CONTENT-4` | `T2.2` |
| `CONTENT-5` | `T2.2` |
| `CONTENT-6` | `T2.3` |
| `CONTENT-7` | `T2.1`, `T2.2` |
| `TEST-3` | `T2.5` |

---

## T2.1 — Tambah `ModuleContext` dan `ResolvedContent`

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** `T1.3`
- **Trace:** `CONTENT-7`, `MOD-5`
- **Referensi:** `crates/island-core/src/view_engine.rs:21-32`, `state.rs:98-109`.
- **Langkah:**
  1. Definisikan `ModuleContext` read-only dengan snapshot domain yang relevan.
  2. Definisikan `ContentReason` (`Priority`, `UserPanel`, `UserSelector`, `Fallback`, `UnsupportedTier`).
  3. Definisikan `ResolvedContent`.
  4. Jangan memasukkan `AppState` penuh ke context.
- **Acceptance:**
  - Module tidak dapat memutasi state.
  - Context mudah dibangun dari snapshot.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `SlotContext` diganti menjadi `ModuleContext` read-only dengan tambahan `source_selector_open`; alias `SlotContext` dipertahankan untuk module existing. `ContentReason` dan `ResolvedContent` ditambahkan di `content_resolver.rs` dan diekspor dari core. Context dibangun dari snapshot `AppState`, bukan `&AppState`.

## T2.2 — Implementasikan `ContentResolver`

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** `T2.1`, `T1.2`
- **Trace:** `CONTENT-1`–`CONTENT-5`, `CONTENT-7`
- **Langkah:**
  1. Filter module berdasarkan tier view aktif.
  2. Hitung priority masing-masing module.
  3. Terapkan override prayer panel dan source selector.
  4. Pilih priority tertinggi; tie-break dengan urutan registrasi.
  5. Fallback ke System jika semua `INACTIVE`.
  6. Sertakan reason pada hasil.
- **Acceptance:**
  - Resolver deterministik.
  - Test matrix module × tier × override lulus.
- **Risiko:** Sedang–tinggi; gunakan characterization test.
- **Catatan verifikasi:** `ContentResolver` memfilter descriptor berdasarkan tier, menerapkan override prayer/source sebagai priority efektif, memilih priority tertinggi dengan tie-break urutan registrasi (kandidat pertama menang), dan fallback ke System. `ContentReason` membedakan `Priority`, `UserPanel`, `UserSelector`, `Fallback`, dan `UnsupportedTier`. `ViewEngine` menulis module aktif hanya dari `ResolvedContent`; `update_active_module` memakai resolver. Test unit priority, override, fallback tier, dan alert-vs-override lulus.

## T2.3 — Pindahkan aturan konten dari Slint

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** `T2.2`
- **Trace:** `CONTENT-6`
- **Referensi:** `wide-view.slint:15-31`, `expand-view.slint:15-20`, `compact-view.slint:6-18`.
- **Langkah:**
  1. Hapus `show-system`, `show-media`, `show-prayer`, dan `show-source` sebagai aturan bisnis.
  2. Ganti dengan properti hasil resolver yang sudah final.
  3. Pertahankan animasi/visibility presentasi, bukan keputusan module.
- **Acceptance:**
  - View tidak lagi membandingkan `active-module` untuk menentukan konten.
  - Perilaku visual existing setara.
- **Risiko:** Tinggi; lakukan smoke test per view.
- **Catatan verifikasi:** `wide-view.slint` dan `expand-view.slint` hanya membaca `wide-module` / `expand-module` hasil resolver. `is-playing` tersisa hanya untuk animasi/ikon di dalam slot media. Source selector tetap overlay presentasi eksplisit; tidak ada fallback module di view.

## T2.4 — Hapus jalur override langsung di reducer

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T2.2`
- **Trace:** `CONTENT-1`, `STATE-2`
- **Referensi:** `reducer.rs:288-293`, `reducer.rs:535-538`.
- **Langkah:**
  1. Ganti penulisan `state.active_module = ...` dengan pemanggilan resolver.
  2. Pastikan `ModuleSwitchRequested` tetap menghormati tier dan validasi module.
- **Acceptance:**
  - Tidak ada penulisan module aktif selain lewat resolver.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `ToggleSourceSelector` selalu memicu resolver, termasuk saat selector ditutup; menutup selector pada wide paused media kembali ke System. `ModuleSwitchRequested` hanya memvalidasi module/tier lalu memanggil resolver, sehingga request tidak dapat memaksa module yang kalah prioritas atau tidak mendukung tier. Tidak ada write langsung ke `engine.active_module`; field engine menjadi `pub(crate)`. Test characterization sumber selector diperbarui dan test request resolver ditambahkan.

## T2.5 — Test regresi konten

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** `T2.3`
- **Trace:** `TEST-3`, `CONTENT-1`–`CONTENT-7`
- **Langkah:**
  1. Test media playing/paused di Compact/Wide/Expand.
  2. Test alert charging/prayer.
  3. Test prayer panel dan source selector.
  4. Test fallback module tanpa slot.
- **Acceptance:**
  - Semua kombinasi penting tercakup dan sesuai baseline/PRD.
  - Setiap test memeriksa module dan `ContentReason`.
- **Risiko:** Rendah.
- **Catatan verifikasi:** `crates/island-core/tests/content_resolution.rs` mencakup media playing/paused, media idle, alert, prayer panel, source selector, override tidak didukung tier, dan tie-break registrasi. Setiap test memeriksa module dan `ContentReason`. Fallback module tanpa slot tetap dites di `island-ui` (`missing_tier_and_unknown_module_fall_back_to_system`). `cargo test --workspace` lulus 157 test.
