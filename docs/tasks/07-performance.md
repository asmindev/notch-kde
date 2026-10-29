# Tasks — Performance

Fase: 7  
PRD: [`../prd/07-performance.md`](../prd/07-performance.md)  
Render policy: [`09-render-policy.md`](09-render-policy.md)  
Index: [`../TASKS.md`](../TASKS.md)

---

## Trace

| Requirement | Task |
|---|---|
| `PERF-1` | `T7.1` |
| `PERF-2` | `T7.2` |
| `PERF-3` | `T7.3` |
| `PERF-4` | `TR4`, `T7.4` |
| `PERF-5` | `T7.5` |
| `VIEW-5` | `TR4`, `T7.4` |
| `VIEW-6` | `T7.5` |
| `RENDER-5` | `TR4`, `T7.4` |

---

## T7.1 — Cache album art

- **Status:** `[ ]`
- **Prioritas:** P2
- **Dependency:** `T3.3`, `T6.4`
- **Trace:** `PERF-1`
- **Referensi:** `crates/island-ui/src/view_model.rs:60-75`.
- **Langkah:**
  1. Simpan cache `art_url -> Image`/hasil load.
  2. Hanya reload jika path berubah atau file berubah.
  3. Batasi ukuran cache.
- **Acceptance:**
  - Spectrum update tidak memicu load album art.
- **Risiko:** Sedang.

## T7.2 — Cache jadwal prayer dan model Slint

- **Status:** `[ ]`
- **Prioritas:** P2
- **Dependency:** `T7.1`
- **Trace:** `PERF-2`
- **Referensi:** `view_model.rs:168-223`.
- **Langkah:**
  1. Hitung jadwal hanya saat tanggal/kota/settings berubah atau menit berubah.
  2. Simpan `VecModel` dan gunakan kembali jika isi sama.
  3. Hindari `Utc::now()` pada setiap sync.
- **Acceptance:**
  - Update non-prayer tidak menghitung ulang jadwal.
- **Risiko:** Sedang.

## T7.3 — Diff sync per bagian

- **Status:** `[ ]`
- **Prioritas:** P2
- **Dependency:** `T3.3`, `T7.1`, `T7.2`, `TR5`
- **Trace:** `PERF-3`
- **Referensi:** `runtime.rs:684-709`, `view_model.rs:13-103`.
- **Langkah:**
  1. Bandingkan snapshot baru dengan snapshot terakhir per kelompok (navigation, time, media, prayer, settings, pet).
  2. Update context hanya untuk kelompok yang berubah.
  3. Jangan clone state penuh jika tidak perlu.
- **Acceptance:**
  - Update spectrum hanya menyentuh bar spectrum/media terkait.
- **Risiko:** Sedang–tinggi; butuh cache key yang benar.

## T7.4 — Validasi gating timer pada slot tidak aktif

- **Status:** `[ ]`
- **Prioritas:** P2
- **Dependency:** `T2.3`, `TR4`
- **Trace:** `PERF-4`, `VIEW-5`, `RENDER-5`
- **Referensi:** `alert-compact-slot.slint:23-27`, `prayer-expand-slot.slint:24-28`.
- **Langkah:**
  1. Audit setiap slot setelah `TR4` untuk memastikan timer animasi hanya berjalan saat slot aktif/terlihat.
  2. Perbaiki slot yang terlewat oleh `RenderPolicy`.
  3. Kecualikan timer yang memang dibutuhkan untuk transisi.
- **Acceptance:**
  - Tidak ada timer yang berjalan sia-sia di slot hidden.
  - Cakupan slot dicek per tier dan per module.
- **Risiko:** Rendah.

## T7.5 — Ukur dan dokumentasikan hasil performa

- **Status:** `[ ]`
- **Prioritas:** P2
- **Dependency:** `T7.3`, `TR6`
- **Trace:** `PERF-5`, `VIEW-6`
- **Langkah:**
  1. Ukur update cost sebelum/sesudah dengan profiling sederhana.
  2. Catat hasil di PRD atau komentar task.
  3. Pastikan tidak ada regresi visual, termasuk fallback pet.
- **Acceptance:**
  - Ada bukti kuantitatif atau observasi yang jelas.
- **Risiko:** Rendah.
