# PRD — Performance

Status: Draft  
Domain: Performance  
Requirement: `PERF-*`  
Task: [`../tasks/07-performance.md`](../tasks/07-performance.md)  
Induk: [`../PRD.md`](../PRD.md)

---

## 1. Tujuan Domain

Mengurangi pekerjaan UI yang tidak perlu tanpa mengubah perilaku visual. Fokus utama: album art, jadwal prayer, model Slint, dan timer slot yang tidak aktif.

Keputusan kapan render/pet/spectrum/timer boleh aktif dimiliki [`10-render-policy.md`](10-render-policy.md). Domain ini fokus pada cache, diff sync, dan validasi cakupan slot.

## 2. Temuan Saat Ini

- `ViewModel::sync_state_to_ui` dipanggil pada setiap update runtime.
- Album art dimuat ulang dari disk pada setiap sync, berpotensi setiap frame spectrum CAVA.
- Jadwal prayer dihitung ulang pada setiap sync.
- Model Slint dibangun ulang meskipun isinya sama.
- Timer animasi di slot hidden dapat terus berjalan.

Referensi: `crates/island-ui/src/view_model.rs:60-75`, `view_model.rs:168-223`, `apps/island-app/src/runtime.rs:684-709`, `crates/island-ui/ui/modules/alert/alert-compact-slot.slint:23-27`, `prayer-expand-slot.slint:24-28`.

## 3. Requirement

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `PERF-1` | Album art tidak dimuat ulang jika `art_url` tidak berubah. | P2 | `T7.1` |
| `PERF-2` | Jadwal prayer dan model Slint tidak dihitung ulang pada event yang tidak relevan. | P2 | `T7.2` |
| `PERF-3` | Update spectrum CAVA tidak memicu rebuild model yang tidak berubah. | P2 | `T7.3` |
| `PERF-4` | Slot tidak aktif tidak menjalankan timer animasi yang tidak terlihat, kecuali diperlukan untuk transisi. Implementasi mengikuti `RenderPolicy` di `TR4`; `T7.4` memvalidasi cakupan slot. | P2 | `TR4`, `T7.4` |
| `PERF-5` | Hasil optimasi diukur dan didokumentasikan. | P2 | `T7.5` |

## 4. Strategi

```text
ViewSnapshot
  -> UiSyncEngine
       -> cache key per kelompok
       -> update hanya kelompok yang berubah
       -> image cache untuk album art
       -> model cache untuk prayer/spectrum
```

Cache key minimal:

| Kelompok | Key |
|---|---|
| Album art | path/URL + mtime opsional |
| Prayer | tanggal + kota + method + settings terkait |
| Time | menit/format |
| Media | title/artist/status/position bucket |
| Settings | versi settings |
| Navigation | tier/visibility/overlap |

## 5. Acceptance Criteria

- Update spectrum tidak memuat ulang album art.
- Update non-prayer tidak menghitung ulang jadwal prayer.
- Model yang isinya sama tidak di-rebuild.
- Slot hidden tidak menjalankan timer animasi yang tidak perlu.
- Tidak ada regresi visual pada smoke test.
- Ada catatan hasil pengukuran sebelum/sesudah.

## 6. Test dan Pengukuran

| Test/Ukur | Cakupan |
|---|---|
| Art cache hit | `PERF-1` |
| Prayer cache hit | `PERF-2` |
| Model reuse | `PERF-3` |
| Timer gating | `PERF-4` |
| Profiling update spectrum | `PERF-5` |

Test unit dapat memakai cache key dan fake loader. Pengukuran runtime dilakukan manual pada session KDE.

## 7. Out of Scope

- GPU profiling mendalam.
- Mengganti renderer Slint.
- Mengubah desain animasi.
- Menambah cache persisten antar-restart.

## 8. Risiko

| Risiko | Mitigasi |
|---|---|
| Cache key salah → tampilan stale | Key eksplisit dan test invalidation |
| Cache tidak terbatas | Batasi ukuran dan LRU sederhana |
| Diff terlalu granular | Mulai per kelompok, bukan per field |
| Optimasi mengubah timing animasi | Jangan ubah durasi; hanya gating slot hidden |
