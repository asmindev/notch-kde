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

- **Status:** `[x]`
- **Prioritas:** P2
- **Dependency:** `T3.3`, `T6.4`
- **Trace:** `PERF-1`
- **Referensi:** `crates/island-ui/src/view_model.rs` (`ArtCache`, `art_key`).
- **Langkah:**
  1. Simpan cache `art_url -> Image`/hasil load.
  2. Hanya reload jika path berubah atau file berubah.
  3. Batasi ukuran cache.
- **Acceptance:**
  - Spectrum update tidak memicu load album art.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `ArtCache` memakai key path + mtime + length, kapasitas 8 entri, dan mengembalikan `Unchanged` untuk URL/path yang sama. Path kosong atau hilang menghasilkan `Update(None)` sekali. Test 100 iterasi berulang membuktikan tidak ada load ulang.

## T7.2 — Cache jadwal prayer dan model Slint

- **Status:** `[x]`
- **Prioritas:** P2
- **Dependency:** `T7.1`
- **Trace:** `PERF-2`
- **Referensi:** `crates/island-ui/src/view_model.rs` (`PrayerCache`, `sync_prayer_schedule`).
- **Langkah:**
  1. Hitung jadwal hanya saat tanggal/kota/settings berubah atau menit berubah.
  2. Simpan `VecModel` dan gunakan kembali jika isi sama.
  3. Hindari `Utc::now()` pada setiap sync.
- **Acceptance:**
  - Update non-prayer tidak menghitung ulang jadwal.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `PrayerCache` memakai key kota + tanggal + menit; label kota disimpan sekali sebagai `ModelRc`. Schedule tidak dihitung ulang saat key sama. Test 100 iterasi pada menit yang sama hanya melakukan satu refresh.

## T7.3 — Diff sync per bagian

- **Status:** `[x]`
- **Prioritas:** P2
- **Dependency:** `T3.3`, `T7.1`, `T7.2`, `TR5`
- **Trace:** `PERF-3`
- **Referensi:** `apps/island-app/src/runtime.rs` (event dispatch), `crates/island-ui/src/view_model.rs` (`UiSyncCache`, `SyncGroups`).
- **Langkah:**
  1. Bandingkan snapshot baru dengan snapshot terakhir per kelompok (navigation, time, media, prayer, settings, pet).
  2. Update context hanya untuk kelompok yang berubah.
  3. Jangan clone state penuh jika tidak perlu.
- **Acceptance:**
  - Update spectrum hanya menyentuh bar spectrum/media terkait.
- **Risiko:** Sedang–tinggi; butuh cache key yang benar.
- **Catatan verifikasi:** `SyncGroups::between` membandingkan snapshot terakhir per kelompok navigation/time/battery/media/prayer/settings/presentation. `UiSyncCache` menyimpan state terakhir dan cache model. Event spectrum tetap memakai fast path, sedangkan full sync hanya mengirim kelompok yang berubah; album art dan player sources memakai cache.

## T7.4 — Validasi gating timer pada slot tidak aktif

- **Status:** `[x]`
- **Prioritas:** P2
- **Dependency:** `T2.3`, `TR4`
- **Trace:** `PERF-4`, `VIEW-5`, `RENDER-5`
- **Referensi:** `crates/island-ui/ui/modules/alert/alert-compact-slot.slint`, `crates/island-ui/ui/modules/prayer/*.slint`, komponen pet.
- **Langkah:**
  1. Audit setiap slot setelah `TR4` untuk memastikan timer animasi hanya berjalan saat slot aktif/terlihat.
  2. Perbaiki slot yang terlewat oleh `RenderPolicy`.
  3. Kecualikan timer yang memang dibutuhkan untuk transisi.
- **Acceptance:**
  - Tidak ada timer yang berjalan sia-sia di slot hidden.
  - Cakupan slot dicek per tier dan per module.
- **Risiko:** Rendah.
- **Catatan verifikasi:** Audit seluruh `Timer` di `crates/island-ui/ui`: timer periodik digate oleh `root.active && IslandContext.render-animations`, ditambah kondisi domain masing-masing (charging, blink, shake, phase, alert pulse). Timer `blink-close` memakai `running: false` dan dipicu manual. Tidak ada timer slot hidden yang berjalan periodik; tidak diperlukan perubahan Slint.

## T7.5 — Ukur dan dokumentasikan hasil performa

- **Status:** `[x]`
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
- **Catatan verifikasi:** Test cache menunjukkan 100 request berulang hanya memicu satu load/refresh, dan jalur spectrum memakai update bar tanpa full sync. Smoke run KDE Wayland 30 detik berhasil start dengan WGPU; `RenderStats` mencatat `ui_syncs_per_sec=0.27`, `pet_ticks_per_sec=1.33`, `pet_redraws_per_sec=1.33`. Fallback vector tidak dipicu pada run ini; perubahan tidak menyentuh geometri maupun jalur render pet.
