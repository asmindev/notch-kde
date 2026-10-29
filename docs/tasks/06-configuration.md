# Tasks — Configuration

Fase: 6  
PRD: [`../prd/06-configuration.md`](../prd/06-configuration.md)  
Index: [`../TASKS.md`](../TASKS.md)

---

## Trace

| Requirement | Task |
|---|---|
| `CFG-1` | `T6.1` |
| `CFG-2` | `T6.1`, `T6.2` |
| `CFG-3` | `T6.1` |
| `CFG-4` | `T6.3` |
| `CFG-5` | `T6.2` |
| `CFG-6` | `T6.5` |
| `CFG-7` | `T6.4` |
| `CFG-8` | `T6.2`, `T6.5` |
| `ERR-1` | `T6.6` |
| `TEST-6` | `T6.7` |

---

## T6.1 — Definisikan `AppConfig`

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T5.6`
- **Trace:** `CFG-1`, `CFG-2`, `CFG-3`
- **Referensi:** `settings_store.rs`, `settings.rs`, `cava.rs`, `window.rs`, `runtime.rs`.
- **Langkah:**
  1. Buat `AppConfig` untuk window spec, dodge enable, path config, service enable, CAVA.
  2. Beri default yang sama dengan perilaku saat ini.
  3. Jangan mencampur dengan `PersistedSettings`.
- **Acceptance:**
  - Config deployment dapat diubah tanpa menyentuh user settings.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `AppConfig`, `WindowSpec`, `CavaConfig`, dan `ServiceToggles` berada di `apps/island-app/src/config.rs`; `PersistedSettings` tetap hanya berisi preferensi user. Default mempertahankan 480x208, compact 156, wide 390, expand 440, dan CAVA aktif.

## T6.2 — Buat `ConfigLoader`

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T6.1`
- **Trace:** `CFG-2`, `CFG-5`, `CFG-8`
- **Langkah:**
  1. Muat `AppConfig` dari `$XDG_CONFIG_HOME/notchde/config.json` jika ada.
  2. Terapkan override env developer secara eksplisit.
  3. Validasi dan fallback ke default dengan warning.
  4. Kembalikan `Result` atau config tervalidasi, bukan panic.
- **Acceptance:**
  - Config malformed tidak membuat panic.
  - Path yang tidak valid tercatat.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `ConfigLoader` membaca `$XDG_CONFIG_HOME/notchde/config.json` (fallback `$HOME/.config/notchde/config.json`), menerapkan `NOTCHDE_CONFIG_DIR`, `NOTCHDE_CAVA_CONFIG`, `ISLAND_DODGE`, dan `PILL_DODGE`, lalu memvalidasi config. JSON malformed, path gagal dibaca, atau nilai di luar batas menghasilkan default + warning tanpa panic.

## T6.3 — Pindahkan `ISLAND_DODGE` dari core

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T6.2`
- **Trace:** `CFG-4`
- **Referensi:** `crates/island-core/src/transition_policy.rs:25`, `crates/island-platform/src/overlap.rs:60-61`.
- **Langkah:**
  1. Hapus pembacaan env dari `island-core`.
  2. Teruskan `dodge_enabled` sebagai parameter config ke policy/overlap.
  3. Pastikan core tetap deterministik.
- **Acceptance:**
  - `island-core` tidak membaca environment.
- **Risiko:** Rendah.
- **Catatan verifikasi:** `transition_policy.rs` tidak lagi membaca environment; `dodge_enabled` diteruskan melalui `ViewEngine` dan `OverlapDetector::start`. `rg` memastikan `island-core` tidak memiliki pembacaan `std::env`.

## T6.4 — Sentralisasi `WindowSpec` dan geometry

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T6.1`
- **Trace:** `CFG-7`
- **Referensi:** `runtime.rs:17-37`, `input_region.rs:17-53`, `bootstrap.rs:19-26`, `metrics.slint:3-30`.
- **Langkah:**
  1. Definisikan `WindowSpec` untuk ukuran window, compact, alert, wide, expand, ear, touch height.
  2. Gunakan spec yang sama untuk input region.
  3. Dokumentasikan hubungan dengan `ThemeMetrics`.
  4. Jika perlu, hasilkan nilai Slint dari spec atau sebaliknya melalui satu lokasi.
- **Acceptance:**
  - Tidak ada angka 480/156/390/440 yang terduplikasi tanpa alasan.
  - Tidak ada mismatch hit region dan tampilan.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `WindowSpec` menjadi sumber ukuran window/compact/alert/wide/expand/touch/ear. Nilai diterapkan ke `ThemeMetrics` sebelum window ditampilkan, dipakai `input_region::rect_for_state`, `Bootstrap`, dan watcher geometry. Nilai dimensi di `metrics.slint` dihapus agar tidak ada duplikasi 480/156/390/440.

## T6.5 — Implementasikan migration settings

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T6.2`
- **Trace:** `CFG-6`, `CFG-8`
- **Referensi:** `settings.rs:6`, `settings.rs:124-178`.
- **Langkah:**
  1. Baca `version` dari file.
  2. Tambahkan fungsi migrasi per versi.
  3. Jika versi tidak dikenal, gunakan default + warning, jangan panic.
  4. Persist versi baru setelah migrasi sukses.
- **Acceptance:**
  - Settings lama tetap dapat dimuat.
  - Test migration lulus.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `migrate_settings_value` membaca `version`, memigrasikan settings tanpa versi ke versi saat ini, dan menolak versi masa depan tanpa panic. `SettingsStore::load` menyimpan ulang file setelah migrasi sukses.

## T6.6 — Hapus panic pada platform preflight

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T6.2`
- **Trace:** `ERR-1`
- **Referensi:** `crates/island-platform/src/kde/mod.rs:27-29`.
- **Langkah:**
  1. Ubah `get_primary_screen_geometry` agar mengembalikan `Result`/`Option`.
  2. Bootstrap memakai fallback geometry.
  3. Catat warning jika `kscreen-doctor` tidak tersedia.
- **Acceptance:**
  - Aplikasi tidak panic di lingkungan tanpa tool KDE.
- **Risiko:** Rendah.
- **Catatan verifikasi:** `KdeManager::get_primary_screen_geometry` mengembalikan `Option`; bootstrap/runtime memakai `ScreenGeometry::fallback()` dan mencatat warning bila `kscreen-doctor`/`qdbus` tidak tersedia.

## T6.7 — Test konfigurasi

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T6.5`
- **Trace:** `TEST-6`
- **Langkah:**
  1. Test default.
  2. Test partial JSON.
  3. Test malformed JSON.
  4. Test out-of-range values.
  5. Test migration.
  6. Test env override hanya di layer yang diizinkan.
- **Acceptance:**
  - Cakupan config memadai.
- **Risiko:** Rendah.
- **Catatan verifikasi:** Test mencakup default, partial JSON, malformed JSON, nilai di luar batas, override env di loader, fallback geometry, migrasi versi lama, dan versi masa depan. `cargo test --workspace` lulus 187 test.
