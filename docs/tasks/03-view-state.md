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

- **Status:** `[ ]`
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

## T3.2 — Jadikan `ViewEngine` canonical untuk navigasi

- **Status:** `[ ]`
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

## T3.3 — Tambah snapshot `ViewSnapshot`

- **Status:** `[ ]`
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

## T3.4 — Test invariant state

- **Status:** `[ ]`
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
