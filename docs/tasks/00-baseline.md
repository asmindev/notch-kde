# Tasks — Baseline & Quality

Fase: 0  
PRD: [`../prd/08-quality.md`](../prd/08-quality.md)  
Index: [`../TASKS.md`](../TASKS.md)

---

## Trace

| Requirement | Task |
|---|---|
| `BASE-1` | `T0.1` |
| `BASE-2` | `T0.2` |
| `BASE-3` | `T0.3` |
| `BASE-4` | `T0.4` |
| `TEST-1` | `T0.1` |
| `TEST-2` | `T0.4` |

---

## T0.1 — Migrasikan atau hapus test root orphan

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** —
- **Trace:** `BASE-1`, `TEST-1`
- **Masalah:** `tests/` di root tidak termasuk workspace member sehingga tidak dijalankan `cargo test --workspace`. Isinya menduplikasi test core dan sebagian sudah bertentangan.
- **Referensi:** `Cargo.toml:1-10`, `tests/view_routing.rs:26-28`, `crates/island-core/tests/view_routing.rs:26-39`.
- **Langkah:**
  1. Bandingkan setiap test root dengan test core yang setara.
  2. Pindahkan test yang masih relevan ke `crates/island-core/tests/` dengan nama unik.
  3. Hapus test yang stale/duplikat.
  4. Pastikan tidak ada folder `tests/` root tanpa package.
- **Acceptance:**
  - `cargo test --workspace` menjalankan semua test yang relevan.
  - Tidak ada test dengan ekspektasi collapse yang bertentangan.
- **Risiko:** Rendah.
- **Catatan verifikasi:** Root `tests/` dihapus karena seluruh isinya sudah tercakup versi terbaru di `crates/island-core/tests/`. Saat task ini, `cargo test --workspace` lulus 120 test; `cargo clippy --workspace --all-targets` selesai dengan warning pre-existing. `cargo fmt --all -- --check` masih gagal karena format pre-existing di file di luar task.

## T0.2 — Rekonsiliasi API mati dan policy yang bertentangan

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** `T0.1`
- **Trace:** `BASE-2`
- **Masalah:** Beberapa API tidak dipakai dan salah satunya bertentangan dengan perilaku aktual.
- **Referensi:** `crates/island-core/src/view_engine.rs:391-400`, `transition_policy.rs:37-69`, `events.rs:99`.
- **Langkah:**
  1. Hapus `ViewEngine::handle_media_update` jika tidak dipakai.
  2. Hapus `TransitionPolicy::resolve_view_on_click` dan `resolve_view_on_hover` jika tidak dipakai, atau perbaiki agar konsisten dengan `ViewEngine`.
  3. Putuskan: implementasikan `Effect::CancelTimer` di Phase 4, atau hapus dari enum sampai dibutuhkan.
- **Acceptance:**
  - Tidak ada API publik yang menyesatkan tentang perilaku collapse.
  - Search tidak menemukan pemakaian method yang dihapus.
- **Risiko:** Rendah.
- **Catatan verifikasi:** `ViewEngine::handle_media_update`, `TransitionPolicy::resolve_view_on_click`, dan `TransitionPolicy::resolve_view_on_hover` dihapus. `Effect::CancelTimer` dihapus dari enum sampai dibutuhkan; `T4.2` akan menambahkan kembali cancel melalui `TimerScheduler`. Wildcard match effect di runtime ikut dihapus agar tidak ada warning unreachable.

## T0.3 — Bersihkan dependency workspace

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** —
- **Trace:** `BASE-3`
- **Masalah:** `island-services` mendeklarasikan `island-platform` tanpa penggunaan.
- **Referensi:** `crates/island-services/Cargo.toml`, `rg "island_platform" crates/island-services`.
- **Langkah:**
  1. Konfirmasi tidak ada import `island_platform` di `island-services`.
  2. Hapus dependency dari `Cargo.toml`.
  3. Jalankan build/test.
- **Acceptance:**
  - Dependency graph lebih kecil.
  - Build dan test tetap lulus.
- **Risiko:** Rendah.
- **Catatan verifikasi:** `island-platform` dihapus dari `crates/island-services/Cargo.toml`; tidak ada import `island_platform` di crate tersebut. Saat task ini, `cargo test --workspace` lulus 120 test dan clippy selesai dengan warning pre-existing.

## T0.4 — Tambah characterization test untuk perilaku rawan

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** `T0.1`
- **Trace:** `BASE-4`, `TEST-2`
- **Masalah:** Perubahan resolver nanti berisiko mengubah perilaku media paused, prayer di compact, dan staging collapse.
- **Referensi:** `reducer.rs:267-301`, `view_engine.rs:88-120`, `wide-view.slint:15-31`, `compact-view.slint:6-18`.
- **Langkah:**
  1. Tambah test media paused dengan judul tidak kosong di Compact dan Wide.
  2. Tambah test prayer panel saat view Compact.
  3. Tambah test source selector saat media paused.
  4. Tambah test staging collapse Wide → Compact via timer.
- **Acceptance:**
  - Test mendokumentasikan perilaku saat ini, termasuk inkonsistensi yang diketahui.
  - Test diberi tag/nama `characterization` bila perlu.
- **Risiko:** Rendah.
- **Catatan verifikasi:** Media paused Compact/Wide, prayer panel Compact, dan staging collapse sudah tercakup test existing. Ditambahkan `test_source_selector_keeps_paused_media_active_in_wide`, yang juga mendokumentasikan bahwa menutup source selector tidak langsung menghitung ulang module sampai transisi berikutnya. `cargo test --workspace` lulus 121 test.

---

## Checklist Sebelum Commit

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets`
- [ ] `cargo test --workspace`
- [ ] Tidak ada perubahan perilaku di luar task
- [ ] Test baru mencakup logika yang berubah
- [ ] Dokumentasi/PRD diperbarui jika ada keputusan baru
- [ ] Tidak menyentuh file yang tidak terkait
- [ ] Tidak commit kecuali diminta
