# PRD — Baseline & Quality

Status: Draft  
Domain: Baseline & quality  
Requirement: `BASE-*`, `TEST-*`, `ERR-*`  
Task: [`../tasks/00-baseline.md`](../tasks/00-baseline.md)  
Induk: [`../PRD.md`](../PRD.md)

---

## 1. Tujuan Domain

Memastikan baseline test dan error handling dapat dipercaya sebelum refactor besar. Menghapus test orphan, API mati, dependency stale, dan panic pada jalur startup.

## 2. Temuan Saat Ini

- `tests/` di root bukan bagian workspace/package sehingga tidak dijalankan.
- Test root menduplikasi test core dan sebagian bertentangan.
- `ViewEngine::handle_media_update` dan `TransitionPolicy::resolve_view_on_*` tidak dipakai.
- `Effect::CancelTimer` dideklarasikan tetapi tidak dipakai.
- `island-services` mendeklarasikan `island-platform` tanpa penggunaan.
- `KdeManager::get_primary_screen_geometry()` memakai `.expect(...)`.
- Error service tidak memiliki health state terpusat.

Referensi: `Cargo.toml`, `tests/`, `crates/island-core/tests/`, `view_engine.rs:391-400`, `transition_policy.rs:37-69`, `events.rs:99`, `crates/island-services/Cargo.toml`, `crates/island-platform/src/kde/mod.rs:27-29`.

## 3. Requirement — Baseline

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `BASE-1` | Test root orphan dihapus atau dipindahkan menjadi test package yang benar. | P0 | `T0.1` |
| `BASE-2` | API mati dan policy yang bertentangan direkonsiliasi. | P0 | `T0.2` |
| `BASE-3` | Dependency workspace yang tidak dipakai dihapus. | P0 | `T0.3` |
| `BASE-4` | Perilaku collapse, media paused, prayer compact, dan source selector memiliki characterization test. | P0 | `T0.4` |

## 4. Requirement — Testing

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `TEST-1` | `cargo test --workspace` menjalankan semua test yang relevan. | P0 | `T0.1` |
| `TEST-2` | Characterization test mencakup perilaku rawan sebelum refactor. | P0 | `T0.4` |
| `TEST-3` | `ContentResolver` memiliki test matrix module × tier × override. | P0 | `T2.5` |
| `TEST-4` | `TimerScheduler` memiliki test cancel dan generation. | P1 | `T4.4` |
| `TEST-5` | `ServiceSupervisor` memiliki test start, degrade, dan shutdown. | P1 | `T5.7` |
| `TEST-6` | Config memiliki test default, partial, malformed, out-of-range, dan migration. | P1 | `T6.7` |

## 5. Requirement — Error & Observability

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `ERR-1` | Platform preflight tidak boleh panic; gunakan fallback dan warning. | P1 | `T6.6` |
| `ERR-2` | Error service dicatat sekali dengan konteks service dan alasan. | P1 | `T5.5` |
| `ERR-3` | Health service tersedia di state runtime; opsional ditampilkan di Settings/About. | P1 | `T5.5` |
| `ERR-4` | Log memakai field `service`, `module`, atau `runtime`. | P1 | `T5.5` |

## 6. Definition of Done

Setiap perubahan task harus memenuhi:

- `cargo fmt --all -- --check` lulus.
- `cargo clippy --workspace --all-targets` tidak menambah warning baru.
- `cargo test --workspace` lulus.
- Test baru mencakup logika yang berubah.
- Tidak ada `unwrap`/`expect` baru pada jalur startup/config yang dapat gagal karena lingkungan.
- Logging memakai `tracing` dan konteks yang jelas.
- Tidak ada perubahan file yang tidak terkait task.

## 7. Acceptance Criteria

- Tidak ada test root orphan.
- Tidak ada API publik yang menyesatkan tentang perilaku collapse.
- Dependency graph tidak memiliki dependency stale yang diketahui.
- Baseline test lulus dan dapat dipercaya.
- Tidak ada panic startup karena tool KDE/config.
- Kegagalan service dapat ditelusuri dari health/log.

## 8. Out of Scope

- Coverage threshold formal.
- Fuzzing.
- Visual regression automation.
- CI pipeline baru, kecuali diminta terpisah.

## 9. Risiko

| Risiko | Mitigasi |
|---|---|
| Menghapus test root menghilangkan cakupan | Bandingkan dengan test core sebelum hapus |
| Characterization test mengunci bug | Tandai sebagai characterization; perbarui saat bug sengaja diperbaiki |
| Terlalu banyak test rapuh | Fokus pada reducer/resolver murni, hindari timing nyata |
