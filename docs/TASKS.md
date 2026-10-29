# Task Breakdown — Modularisasi Arsitektur notchde

Status: Draft  
Tanggal: 2026-10-29  
PRD: [`PRD.md`](PRD.md)  
Traceability: [`TRACEABILITY.md`](TRACEABILITY.md)  
Aturan kerja: [`../AGENTS.md`](../AGENTS.md)

---

## 1. Cara Membaca Dokumen

Task dipecah per domain agar setiap pekerjaan dapat dilacak ke requirement PRD. Dokumen ini adalah index; detail task ada di folder `tasks/`.

| Domain | Requirement | Task | File |
|---|---|---|---|
| Baseline & kualitas | `BASE-*`, `TEST-*`, `ERR-*` | `T0.*` | [`tasks/00-baseline.md`](tasks/00-baseline.md) |
| Module system | `MOD-*` | `T1.*` | [`tasks/01-module-system.md`](tasks/01-module-system.md) |
| Content resolution | `CONTENT-*` | `T2.*` | [`tasks/02-content-resolution.md`](tasks/02-content-resolution.md) |
| View & state | `VIEW-*`, `STATE-*` | `T3.*` | [`tasks/03-view-state.md`](tasks/03-view-state.md) |
| Timer & effect | `TIMER-*` | `T4.*` | [`tasks/04-timers-effects.md`](tasks/04-timers-effects.md) |
| Services | `SVC-*` | `T5.*` | [`tasks/05-services.md`](tasks/05-services.md) |
| Configuration | `CFG-*` | `T6.*` | [`tasks/06-configuration.md`](tasks/06-configuration.md) |
| Performance | `PERF-*` | `T7.*` | [`tasks/07-performance.md`](tasks/07-performance.md) |
| Render & activity | `RENDER-*` | `TR1`–`TR6` | [`tasks/09-render-policy.md`](tasks/09-render-policy.md) |
| Optional cleanup | `OPT-*` | `T8.*` | [`tasks/08-optional-cleanup.md`](tasks/08-optional-cleanup.md) |

Catatan: `TR*` adalah track render yang dikerjakan setelah Phase 0 dan sebelum Phase 1. Nomor `TR` bukan urutan fase.

Aturan:

- Kerjakan task sesuai dependency.
- Satu task idealnya satu perubahan yang dapat direview dan dites.
- Jangan mencampur refactor besar dengan fitur baru.
- Perbarui status di file task domain, bukan hanya di index.
- Jika implementasi menyimpang dari PRD, perbarui PRD atau catat alasan di task.

Legenda status: `[ ]` belum, `[~]` dikerjakan, `[x]` selesai, `[-]` dibatalkan.

---

## 2. Status Board

| Fase | Domain | Task | Status | Gate |
|---|---|---|---|---|
| 0 | Baseline | `T0.1`–`T0.4` | `[x]` | Baseline test terpercaya |
| R | Render activity | `TR1`–`TR6` | `[x]` | Hidden tanpa redraw/tick periodik |
| 1 | Module | `T1.1`–`T1.5` | `[~]` | Module dummy tanpa sentuh ViewEngine |
| 2 | Content | `T2.1`–`T2.5` | `[ ]` | Tidak ada aturan konten di Slint |
| 3 | View/state | `T3.1`–`T3.4` | `[ ]` | Tidak ada state navigasi divergen |
| 4 | Timer/effect | `T4.1`–`T4.4` | `[ ]` | Tidak ada string timer |
| 5 | Services | `T5.1`–`T5.7` | `[ ]` | Semua service start/stop/health |
| 6 | Config | `T6.1`–`T6.7` | `[ ]` | Tidak ada env di core; no panic config |
| 7 | Performance | `T7.1`–`T7.5` | `[ ]` | Tidak ada reload berulang |
| 8 | Optional | `T8.1`–`T8.3` | `[ ]` | Hanya jika dipicu kebutuhan |

---

## 3. Ringkasan Fase

| Fase | Nama | Fokus | Gate |
|---|---|---|---|
| 0 | Baseline hygiene | Test orphan, API mati, dependency | Baseline bersih dan terpercaya |
| R | Render activity | RenderPolicy, gate pet/timer/spectrum, pengukuran | Hidden tanpa render/tick periodik |
| 1 | Module registry | ModuleId, descriptor, registry, UI binding | Module dummy bisa ditambah tanpa ViewEngine |
| 2 | Content resolver | Satu sumber keputusan konten | Tidak ada aturan konten di Slint |
| 3 | State ownership | Canonical view state | Tidak ada state navigasi divergen |
| 4 | Timer & effect | Timer typed, cancel, executor | Tidak ada string timer |
| 5 | Service lifecycle | Service trait, supervisor, shutdown | Semua service start/stop/health |
| 6 | Configuration | AppConfig, WindowSpec, validasi | Tidak ada env di core; no panic config |
| 7 | UI performance | Cache dan diff sync | Tidak ada reload berulang |
| 8 | Optional cleanup | Context/event split, welcome module | Hanya jika dibutuhkan |

---

## 4. Dependency Graph

```text
T0.1 -> T0.2
T0.1 -> T0.4
T0.3 (independent)

T0.4 -> TR1 -> TR2
TR2 -> TR3
TR2 -> TR4
TR2 -> TR5
TR3 + TR4 + TR5 -> TR6
TR6 -> T1.1

T1.1 -> T1.2 -> T1.3 -> T1.4 -> T1.5

T1.3 -> T2.1 -> T2.2 -> T2.3 -> T2.4 -> T2.5

T2.4 -> T3.1 -> T3.2 -> T3.3 -> T3.4

T3.2 -> T4.1 -> T4.2 -> T4.3 -> T4.4

T4.3 -> T5.1 -> T5.2 -> T5.3 -> T5.4
T5.2 -> T5.5
T5.2 -> T5.6 -> T5.7

T5.6 -> T6.1 -> T6.2 -> T6.3
T6.1 -> T6.4
T6.2 -> T6.5 -> T6.7
T6.2 -> T6.6

T3.3 + T6.4 -> T7.1 -> T7.2 -> T7.3
T2.3 + TR4 -> T7.4
TR5 -> T7.3
T7.3 -> T7.5
TR6 -> T7.5

T7.3 -> T8.1
T5.6 -> T8.2
T2.5 -> T8.3
```

---

## 5. Definition of Done Global

Setiap task dianggap selesai jika:

- Perilaku existing tidak berubah, kecuali task memang menetapkan perubahan perilaku.
- `cargo fmt --all -- --check` lulus.
- `cargo clippy --workspace --all-targets` tidak menambah warning baru.
- `cargo test --workspace` lulus.
- Test baru ditambahkan untuk logika yang berubah.
- Tidak ada `unwrap`/`expect` baru pada jalur startup/config yang dapat gagal karena lingkungan.
- Logging memakai `tracing` dan konteks yang jelas.
- Tidak ada perubahan file yang tidak terkait task.

Checklist sebelum commit ada di [`tasks/00-baseline.md`](tasks/00-baseline.md#checklist-sebelum-commit).

---

## 6. Traceability

Peta lengkap requirement → task ada di [`TRACEABILITY.md`](TRACEABILITY.md). Setiap file task juga mencantumkan requirement yang dicakup pada bagian `Trace`.

Aturan trace:

1. Setiap requirement PRD harus memiliki minimal satu task.
2. Setiap task harus mencantumkan minimal satu requirement.
3. Jika task baru tidak memiliki requirement, tambahkan requirement ke PRD terlebih dahulu.
4. Jika requirement dibatalkan, tandai task terkait dan update PRD.
