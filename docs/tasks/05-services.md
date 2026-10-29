# Tasks — Services

Fase: 5  
PRD: [`../prd/05-services.md`](../prd/05-services.md)  
Index: [`../TASKS.md`](../TASKS.md)

---

## Trace

| Requirement | Task |
|---|---|
| `SVC-1` | `T5.1` |
| `SVC-2` | `T5.1`, `T5.6` |
| `SVC-3` | `T5.1` |
| `SVC-4` | `T5.1` |
| `SVC-5` | `T5.2`, `T5.5` |
| `SVC-6` | `T5.2` |
| `SVC-7` | `T5.4` |
| `SVC-8` | `T5.6` |
| `ERR-2` | `T5.5` |
| `ERR-3` | `T5.5` |
| `ERR-4` | `T5.5` |
| `TEST-5` | `T5.7` |

---

## T5.1 — Definisikan kontrak `Service`

- **Status:** `[ ]`
- **Prioritas:** P1
- **Dependency:** `T4.3`
- **Trace:** `SVC-1`–`SVC-4`
- **Referensi:** `crates/island-services/src/lib.rs`, `clock.rs:18-37`, `cava.rs:19-114`.
- **Langkah:**
  1. Buat `Service`, `ServiceContext`, `ServiceHandle`.
  2. Sediakan sinyal shutdown.
  3. Jangan membuat trait async yang kompleks; gunakan spawn internal.
  4. Pastikan service tidak menerima tipe UI/module.
- **Acceptance:**
  - Minimal satu service percontohan dapat start/stop.
  - Kontrak tidak menyebut Slint atau `AppState`.
- **Risiko:** Sedang.

## T5.2 — Buat `ServiceSupervisor`

- **Status:** `[ ]`
- **Prioritas:** P1
- **Dependency:** `T5.1`
- **Trace:** `SVC-5`, `SVC-6`
- **Langkah:**
  1. Daftarkan service saat startup.
  2. Start semua service dan simpan handle.
  3. Sediakan status health.
  4. Sediakan shutdown dengan timeout.
- **Acceptance:**
  - Satu service gagal tidak menghentikan aplikasi.
  - Shutdown join semua service.
- **Risiko:** Sedang–tinggi.

## T5.3 — Migrasikan service satu per satu

- **Status:** `[ ]`
- **Prioritas:** P1
- **Dependency:** `T5.2`
- **Trace:** `SVC-1`–`SVC-8`
- **Langkah:**
  1. Migrasikan Clock.
  2. Migrasikan Battery.
  3. Migrasikan Media.
  4. Migrasikan DND, Audio, Notifications.
  5. Migrasikan Prayer, ScreenSaver, CAVA.
  6. Migrasikan OverlapDetector sebagai platform service bila sesuai.
- **Acceptance:**
  - Tidak ada `tokio::spawn` liar di fungsi start service.
  - Semua service menerima shutdown.
  - Satu service per perubahan.
- **Risiko:** Sedang.

## T5.4 — Hapus global mutable `MediaService`

- **Status:** `[ ]`
- **Prioritas:** P1
- **Dependency:** `T5.3`
- **Trace:** `SVC-7`
- **Referensi:** `crates/island-services/src/media.rs:12-13`.
- **Langkah:**
  1. Pindahkan `ACTIVE_PLAYER` dan sender ke struct instance.
  2. Berikan instance melalui `ServiceContext` atau argumen.
  3. Pastikan test selection tidak bergantung pada global.
- **Acceptance:**
  - Tidak ada static mutable di MediaService.
  - Test media selection dapat berjalan paralel.
- **Risiko:** Sedang.

## T5.5 — Tambah health state dan logging

- **Status:** `[ ]`
- **Prioritas:** P1
- **Dependency:** `T5.2`
- **Trace:** `SVC-5`, `ERR-2`, `ERR-3`, `ERR-4`
- **Langkah:**
  1. Definisikan `ServiceHealth`.
  2. Kirim perubahan health ke runtime.
  3. Tampilkan minimal di log; opsional di Settings/About.
  4. Sertakan field `service` pada log error.
- **Acceptance:**
  - Kegagalan service dapat ditelusuri tanpa membaca semua log acak.
- **Risiko:** Rendah.

## T5.6 — Implementasikan shutdown runtime

- **Status:** `[ ]`
- **Prioritas:** P1
- **Dependency:** `T5.2`
- **Trace:** `SVC-2`, `SVC-8`
- **Langkah:**
  1. Hentikan penerimaan event baru.
  2. Batalkan semua timer.
  3. Kirim sinyal shutdown ke service.
  4. Join dengan timeout.
  5. Persist settings jika dirty.
- **Acceptance:**
  - Tidak ada child process tertinggal.
  - Aplikasi keluar tanpa hang.
- **Risiko:** Sedang.

## T5.7 — Test service supervisor

- **Status:** `[ ]`
- **Prioritas:** P1
- **Dependency:** `T5.6`
- **Trace:** `TEST-5`
- **Langkah:**
  1. Test start semua service.
  2. Test service gagal start → degraded.
  3. Test service gagal runtime → health berubah.
  4. Test shutdown.
- **Acceptance:**
  - Lifecycle terbukti lewat test.
- **Risiko:** Rendah.
