# PRD — View & State

Status: Draft  
Domain: View & state  
Requirement: `VIEW-*`, `STATE-*`  
Task: [`../tasks/03-view-state.md`](../tasks/03-view-state.md)  
Induk: [`../PRD.md`](../PRD.md)

---

## 1. Tujuan Domain

Menetapkan kepemilikan state yang jelas antara `ViewEngine`, `AppState`, dan UI. `ViewEngine` tetap menjadi state machine navigasi; UI menerima snapshot read-only; tidak ada state navigasi yang diduplikasi secara mutable.

## 2. Temuan Saat Ini

- `AppState` memiliki field `mode`, `previous_mode`, `active_module`, `is_visible`, `is_overlapping`, dan `is_hovered`.
- Field tersebut disalin dari `ViewEngine` melalui `sync_engine()`.
- Beberapa reducer menulis langsung ke engine, beberapa menyalin balik ke state.
- `ViewModel` menerima `&AppState` penuh dan membaca field yang seharusnya menjadi tanggung jawab engine/resolver.
- Window settings dan window utama berbagi context besar.

Referensi: `crates/island-core/src/state.rs:21-124`, `crates/island-core/src/view_engine.rs:21-54`, `crates/island-ui/src/view_model.rs:13-103`.

## 3. Requirement — View

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `VIEW-1` | `ViewEngine` tetap menjadi pemilik state machine tier Compact/Wide/Expand, hover, overlap, reveal, dan dismiss. | P1 | `T3.2` |
| `VIEW-2` | State navigasi tidak diduplikasi sebagai field mutable terpisah; akses via accessor atau snapshot. | P1 | `T3.2` |
| `VIEW-3` | UI menerima `ViewSnapshot` read-only dan tidak memutasi state domain. | P1 | `T3.3` |
| `VIEW-4` | Context window utama dan window settings dipisahkan secara bertahap; settings tidak membawa state domain yang tidak perlu. | P2 | `T8.1` |
| `VIEW-5` | Slot yang tidak aktif menghentikan timer yang tidak perlu. | P2 | `TR4`, `T7.4` |
| `VIEW-6` | Kegagalan render pet tetap memakai fallback vector yang sudah ada. | P1 | `T7.5` |

Gating aktivitas render dan timer mengikuti [`10-render-policy.md`](10-render-policy.md).

## 4. Requirement — State

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `STATE-1` | `AppState` dipecah menjadi sub-state dengan kepemilikan jelas. | P1 | `T3.1` |
| `STATE-2` | Module aktif hanya diubah melalui resolver. | P0 | `T2.4`, `T3.2` |
| `STATE-3` | Setiap event yang mengubah view harus melewati `ViewEngine`. | P1 | `T3.2` |
| `STATE-4` | Invariant visibility, overlap, hover, dan tier memiliki test. | P1 | `T3.4` |

## 5. Target State Ownership

```text
AppState
├── ViewState        -> ViewEngine canonical
├── DomainState      -> media, alert, prayer, battery, dnd
├── ModuleState      -> hasil resolver + override state
├── RuntimeState     -> health, timers, services
└── UserSettings     -> PersistedSettings
```

Aturan:

- `ViewState` tidak menyimpan salinan mutable dari `ViewEngine`.
- `sync_engine()` dihapus atau diubah menjadi accessor tanpa duplikasi.
- UI hanya menerima `ViewSnapshot`.
- `ModuleContext` dibangun dari `DomainState` + `ModuleState`, bukan `AppState` penuh.
- Perubahan preferensi tetap melalui event dan efek persist.

## 6. Kontrak Snapshot

```rust
pub struct ViewSnapshot {
    pub navigation: NavigationSnapshot,
    pub domain: DomainSnapshot,
    pub presentation: PresentationSnapshot,
    pub settings: SettingsSnapshot,
}
```

Snapshot boleh dipecah lebih lanjut untuk diff. Yang penting:

- read-only;
- tidak mengandung handle service/channel;
- dapat dibandingkan untuk caching;
- tidak mengekspos tipe internal Slint.

## 7. Acceptance Criteria

- Tidak ada divergensi antara accessor state dan `ViewEngine`.
- Semua reducer memakai API engine, bukan menulis field navigasi.
- UI tidak dapat mengakses field internal `AppState`.
- Test invariant lulus.
- Perilaku collapse, hover, overlap, dan reveal tidak berubah.

## 8. Test yang Dibutuhkan

| Test | Cakupan |
|---|---|
| Reducer tidak menulis navigasi langsung | `VIEW-1`, `VIEW-2` |
| Active module selalu didukung tier | `STATE-2` |
| Visibility konsisten dengan overlap/hover | `STATE-4` |
| Snapshot tidak berubah saat state tidak berubah | `VIEW-3` |
| Collapse staging Wide → Compact | Regresi `VIEW-1` |

## 9. Out of Scope

- Mengganti reducer dengan state machine library.
- Memindahkan semua state ke satu crate baru.
- Reactive framework selain Slint.

## 10. Risiko

| Risiko | Mitigasi |
|---|---|
| Menghapus field duplikat memecahkan banyak call site | Accessor sementara + test invariant |
| Snapshot terlalu besar | Pecah per kelompok dan diff per bagian |
| UI masih bergantung pada field internal | Larang akses `AppState` dari `island-ui` setelah migrasi |
