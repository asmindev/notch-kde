# Tasks — View & State

Fase: 3  
PRD: [`../prd/03-view-state.md`](../prd/03-view-state.md)  
Index: [`../TASKS.md`](../TASKS.md)

---

## Trace

| Requirement | Task |
|---|---|
| `VIEW-1` | `T3.2` |
| `VIEW-2` | `T3.2` |
| `VIEW-3` | `T3.3` |
| `VIEW-4` | `T8.1` (opsional) |
| `VIEW-5` | `T7.4` |
| `VIEW-6` | `T7.5` |
| `STATE-1` | `T3.1` |
| `STATE-2` | `T2.4`, `T3.2` |
| `STATE-3` | `T3.2` |
| `STATE-4` | `T3.4` |

---

## T3.1 — Pisahkan `AppState` menjadi sub-state

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T2.4`
- **Trace:** `STATE-1`
- **Referensi:** `crates/island-core/src/state.rs:21-86`.
- **Langkah:**
  1. Bentuk `ViewState`, `DomainState`, `ModuleState`, `RuntimeState`, dan `UserSettings`.
  2. Pertahankan field lama sebagai accessor sementara agar test tidak langsung rusak.
  3. Pindahkan state domain sesuai kepemilikan.
- **Acceptance:**
  - Setiap field memiliki pemilik yang jelas.
  - Tidak ada perubahan perilaku.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `AppState` sekarang terdiri dari `view`, `domain`, `module`, `runtime`, `settings`, dan `presentation`. `ViewState` masih memuat salinan navigasi sementara yang disinkronkan dari `ViewEngine`; duplikasi ini dihapus di `T3.2`. Semua call site production/test dimigrasikan ke sub-state yang sesuai. `cargo test --workspace` tetap lulus 157 test.

## T3.2 — Jadikan `ViewEngine` canonical untuk navigasi

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T3.1`
- **Trace:** `VIEW-1`, `VIEW-2`, `STATE-2`, `STATE-3`
- **Referensi:** `state.rs:88-96`, `state.rs:111-124`.
- **Langkah:**
  1. Hapus duplikasi `mode`, `previous_mode`, `active_module`, `is_visible`, `is_overlapping`, `is_hovered` dari state utama, atau ubah menjadi accessor.
  2. Pastikan semua reducer memakai API engine, bukan menulis field.
  3. Tambahkan invariant test.
- **Acceptance:**
  - Tidak ada divergensi antara accessor dan engine.
  - Test state_transitions lulus.
- **Risiko:** Sedang–tinggi.
- **Catatan verifikasi:** `ViewState` sekarang hanya membungkus `ViewEngine`; field salinan `mode`, `previous_mode`, `active_module`, `is_visible`, `is_overlapping`, dan `is_hovered` dihapus. Akses memakai method `ViewState` yang membaca langsung dari engine. `sync_engine()` dihapus; reducer tetap memakai API engine. Test `state_transitions`, `overlap_sequencing`, `view_routing`, dan `slot_priority` lulus.

## T3.3 — Tambah snapshot `ViewSnapshot`

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T3.2`
- **Trace:** `VIEW-3`
- **Langkah:**
  1. Definisikan snapshot read-only untuk UI dan pet.
  2. Pisahkan snapshot navigation, domain, presentation, dan settings.
  3. Ubah `ViewModel` agar menerima snapshot, bukan `AppState` penuh.
- **Acceptance:**
  - UI tidak dapat mengakses field internal state.
  - Snapshot mudah di-diff.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `ViewSnapshot` memiliki `NavigationSnapshot`, `DomainSnapshot`, `PresentationSnapshot`, dan `SettingsSnapshot`, semuanya owned, read-only, dan `PartialEq`. `AppState::snapshot()` membangun snapshot dari sub-state. `ViewModel` dan pet 3D sekarang menerima `&ViewSnapshot`; tidak ada lagi import `AppState` di `island-ui`. Test snapshot stabil dan mendeteksi perubahan state lulus.

## T3.4 — Test invariant state

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T3.2`
- **Trace:** `STATE-4`
- **Langkah:**
  1. Test bahwa setiap event yang mengubah view memanggil resolver.
  2. Test bahwa active module selalu didukung oleh tier aktif.
  3. Test bahwa visibility konsisten dengan overlap/hover.
- **Acceptance:**
  - Invariant utama tercakup.
- **Risiko:** Rendah.
- **Catatan verifikasi:** `crates/island-core/tests/state_invariants.rs` memeriksa bahwa module aktif sama dengan engine dan selalu mendukung tier aktif setelah rangkaian event view, prayer panel dibuang saat pindah ke Compact, serta visibility/overlap/hover konsisten dengan `TransitionPolicy` selama reveal/dismiss. `cargo test --workspace` lulus 161 test.
