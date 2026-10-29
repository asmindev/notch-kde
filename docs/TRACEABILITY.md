# Traceability Matrix — PRD ke Task

Status: Draft  
Tanggal: 2026-10-29  
PRD: [`PRD.md`](PRD.md)  
Tasks: [`TASKS.md`](TASKS.md)

---

## 1. Aturan Trace

- Setiap requirement PRD harus memiliki minimal satu task.
- Setiap task harus mencantumkan requirement pada bagian `Trace`.
- Perubahan requirement harus memperbarui file PRD domain, file task domain, dan matriks ini.
- ID requirement dan task tidak boleh dipakai ulang untuk hal yang berbeda.

## 2. Domain Map

| Domain | Requirement | PRD | Task |
|---|---|---|---|
| Baseline & quality | `BASE-*`, `TEST-*`, `ERR-*` | [`prd/08-quality.md`](prd/08-quality.md) | [`tasks/00-baseline.md`](tasks/00-baseline.md) |
| Module system | `MOD-*` | [`prd/01-module-system.md`](prd/01-module-system.md) | [`tasks/01-module-system.md`](tasks/01-module-system.md) |
| Content resolution | `CONTENT-*` | [`prd/02-content-resolution.md`](prd/02-content-resolution.md) | [`tasks/02-content-resolution.md`](tasks/02-content-resolution.md) |
| View & state | `VIEW-*`, `STATE-*` | [`prd/03-view-state.md`](prd/03-view-state.md) | [`tasks/03-view-state.md`](tasks/03-view-state.md) |
| Timer & effect | `TIMER-*` | [`prd/04-timers-effects.md`](prd/04-timers-effects.md) | [`tasks/04-timers-effects.md`](tasks/04-timers-effects.md) |
| Services | `SVC-*` | [`prd/05-services.md`](prd/05-services.md) | [`tasks/05-services.md`](tasks/05-services.md) |
| Configuration | `CFG-*` | [`prd/06-configuration.md`](prd/06-configuration.md) | [`tasks/06-configuration.md`](tasks/06-configuration.md) |
| Performance | `PERF-*` | [`prd/07-performance.md`](prd/07-performance.md) | [`tasks/07-performance.md`](tasks/07-performance.md) |
| Render & activity | `RENDER-*` | [`prd/10-render-policy.md`](prd/10-render-policy.md) | [`tasks/09-render-policy.md`](tasks/09-render-policy.md) |
| Optional cleanup | `OPT-*` | [`prd/09-optional-cleanup.md`](prd/09-optional-cleanup.md) | [`tasks/08-optional-cleanup.md`](tasks/08-optional-cleanup.md) |

## 3. Requirement → Task

### Baseline & Quality

| Requirement | Task | Verifikasi |
|---|---|---|
| `BASE-1` | `T0.1` | Test root orphan hilang/berpindah |
| `BASE-2` | `T0.2` | API mati dan policy direkonsiliasi |
| `BASE-3` | `T0.3` | Dependency stale dihapus |
| `BASE-4` | `T0.4` | Characterization test tersedia |
| `TEST-1` | `T0.1` | `cargo test --workspace` mencakup semua test |
| `TEST-2` | `T0.4` | Test media paused/prayer/collapse |
| `TEST-3` | `T2.5` | Matriks module × tier × override |
| `TEST-4` | `T4.4` | Test cancel/generation timer |
| `TEST-5` | `T5.7` | Test start/degrade/shutdown service |
| `TEST-6` | `T6.7` | Test config dan migration |
| `ERR-1` | `T6.6` | Tidak ada panic preflight platform |
| `ERR-2` | `T5.5` | Log error service terkontekstual |
| `ERR-3` | `T5.5` | Health state tersedia |
| `ERR-4` | `T5.5` | Field log `service`/`module`/`runtime` |

### Module System

| Requirement | Task | Verifikasi |
|---|---|---|
| `MOD-1` | `T1.1` | `ModuleId` stabil dan unik |
| `MOD-2` | `T1.1`, `T1.3` | Descriptor lengkap per module |
| `MOD-3` | `T1.2` | Registry start sekali |
| `MOD-4` | `T1.2` | Duplikasi ID ditolak |
| `MOD-5` | `T1.1`, `T2.1` | `ModuleContext` read-only |
| `MOD-6` | `T1.4` | Mapping ID → slot per tier |
| `MOD-7` | `T1.4` | Fallback module tanpa slot |

### Content Resolution

| Requirement | Task | Verifikasi |
|---|---|---|
| `CONTENT-1` | `T2.2`, `T2.4` | Hanya resolver yang menulis module aktif |
| `CONTENT-2` | `T2.2` | Filter tier |
| `CONTENT-3` | `T2.2` | Priority + tie-break deterministik |
| `CONTENT-4` | `T2.2` | Fallback System |
| `CONTENT-5` | `T2.2` | Override prayer/source |
| `CONTENT-6` | `T2.3` | Tidak ada aturan konten di Slint |
| `CONTENT-7` | `T2.1`, `T2.2` | `ContentReason` tersedia dan dites |

### View & State

| Requirement | Task | Verifikasi |
|---|---|---|
| `VIEW-1` | `T3.2` | `ViewEngine` canonical |
| `VIEW-2` | `T3.2` | Tidak ada duplikasi state navigasi |
| `VIEW-3` | `T3.3` | UI memakai snapshot read-only |
| `VIEW-4` | `T8.1` | Settings context terpisah (opsional) |
| `VIEW-5` | `T7.4` | Timer slot hidden dihentikan |
| `VIEW-6` | `T7.5` | Smoke test fallback pet |
| `STATE-1` | `T3.1` | Sub-state dengan kepemilikan jelas |
| `STATE-2` | `T2.4`, `T3.2` | Module aktif hanya via resolver |
| `STATE-3` | `T3.2` | Event view melalui engine |
| `STATE-4` | `T3.4` | Test invariant visibility/overlap/hover |

### Timer & Effect

| Requirement | Task | Verifikasi |
|---|---|---|
| `TIMER-1` | `T4.1` | Tidak ada string timer |
| `TIMER-2` | `T4.2` | Schedule/cancel/cancel_all |
| `TIMER-3` | `T4.2` | Generation stale diabaikan |
| `TIMER-4` | `T4.3` | `EffectExecutor` terpisah |
| `TIMER-5` | `T4.3` | Tidak ada duplikasi spawn/sleep/send |

### Services

| Requirement | Task | Verifikasi |
|---|---|---|
| `SVC-1` | `T5.1` | Kontrak `Service` + `ServiceHandle` |
| `SVC-2` | `T5.1`, `T5.6` | Shutdown signal |
| `SVC-3` | `T5.1` | Service tidak tahu UI/module |
| `SVC-4` | `T5.1` | Hanya payload/domain event |
| `SVC-5` | `T5.2`, `T5.5` | Status health |
| `SVC-6` | `T5.2` | Kegagalan service terisolasi |
| `SVC-7` | `T5.4` | Tidak ada global mutable media |
| `SVC-8` | `T5.6` | Shutdown + join timeout |

### Configuration

| Requirement | Task | Verifikasi |
|---|---|---|
| `CFG-1` | `T6.1` | `AppConfig` terpisah |
| `CFG-2` | `T6.1`, `T6.2` | Field deployment lengkap |
| `CFG-3` | `T6.1` | `PersistedSettings` tetap user-only |
| `CFG-4` | `T6.3` | Env tidak dibaca core |
| `CFG-5` | `T6.2` | Validasi + fallback |
| `CFG-6` | `T6.5` | Migration settings |
| `CFG-7` | `T6.4` | Satu sumber geometry |
| `CFG-8` | `T6.2`, `T6.5` | Persist via event/effect |

### Performance

| Requirement | Task | Verifikasi |
|---|---|---|
| `PERF-1` | `T7.1` | Cache album art |
| `PERF-2` | `T7.2` | Cache jadwal/model prayer |
| `PERF-3` | `T7.3` | Diff sync per kelompok |
| `PERF-4` | `TR4`, `T7.4` | Gating timer oleh `RenderPolicy`, divalidasi per slot |
| `PERF-5` | `T7.5` | Pengukuran terdokumentasi |

### Render & Activity

| Requirement | Task | Verifikasi |
|---|---|---|
| `RENDER-1` | `TR1`, `TR2` | Policy murni dan flag context |
| `RENDER-2` | `TR2`, `TR3`, `TR4` | Hidden tanpa redraw/tick/timer periodik |
| `RENDER-3` | `TR4` | Hanya tier aktif di luar crossfade |
| `RENDER-4` | `TR3` | Pet hanya tick saat terlihat |
| `RENDER-5` | `TR4` | Tidak ada timer `\|\| true` |
| `RENDER-6` | `TR5` | Spectrum hanya saat dipakai |
| `RENDER-7` | `TR2`, `TR5` | Hidden surface tidak memicu render |
| `RENDER-8` | `TR6` | Counter dan pengukuran tersedia |

### Optional Cleanup

| Requirement | Task | Verifikasi |
|---|---|---|
| `OPT-1` | `T8.1` | `SettingsContext` terpisah |
| `OPT-2` | `T8.2` | `UiAction`/`DomainEvent` terpisah |
| `OPT-3` | `T8.3` | Keputusan Welcome terdokumentasi |

## 4. Task → Requirement

| Task | Requirement |
|---|---|
| `T0.1` | `BASE-1`, `TEST-1` |
| `T0.2` | `BASE-2` |
| `T0.3` | `BASE-3` |
| `T0.4` | `BASE-4`, `TEST-2` |
| `T1.1` | `MOD-1`, `MOD-2`, `MOD-5` |
| `T1.2` | `MOD-3`, `MOD-4` |
| `T1.3` | `MOD-2` |
| `T1.4` | `MOD-6`, `MOD-7` |
| `T1.5` | `MOD-1`–`MOD-7` |
| `T2.1` | `CONTENT-7`, `MOD-5` |
| `T2.2` | `CONTENT-1`–`CONTENT-5`, `CONTENT-7` |
| `T2.3` | `CONTENT-6` |
| `T2.4` | `CONTENT-1`, `STATE-2` |
| `T2.5` | `TEST-3` |
| `T3.1` | `STATE-1` |
| `T3.2` | `VIEW-1`, `VIEW-2`, `STATE-2`, `STATE-3` |
| `T3.3` | `VIEW-3` |
| `T3.4` | `STATE-4` |
| `T4.1` | `TIMER-1` |
| `T4.2` | `TIMER-2`, `TIMER-3` |
| `T4.3` | `TIMER-4`, `TIMER-5` |
| `T4.4` | `TEST-4` |
| `T5.1` | `SVC-1`–`SVC-4` |
| `T5.2` | `SVC-5`, `SVC-6` |
| `T5.3` | `SVC-1`–`SVC-8` |
| `T5.4` | `SVC-7` |
| `T5.5` | `SVC-5`, `ERR-2`, `ERR-3`, `ERR-4` |
| `T5.6` | `SVC-2`, `SVC-8` |
| `T5.7` | `TEST-5` |
| `T6.1` | `CFG-1`, `CFG-2`, `CFG-3` |
| `T6.2` | `CFG-2`, `CFG-5`, `CFG-8` |
| `T6.3` | `CFG-4` |
| `T6.4` | `CFG-7` |
| `T6.5` | `CFG-6`, `CFG-8` |
| `T6.6` | `ERR-1` |
| `T6.7` | `TEST-6` |
| `T7.1` | `PERF-1` |
| `T7.2` | `PERF-2` |
| `T7.3` | `PERF-3` |
| `T7.4` | `PERF-4`, `VIEW-5`, `RENDER-5` |
| `T7.5` | `PERF-5`, `VIEW-6` |
| `TR1` | `RENDER-1` |
| `TR2` | `RENDER-1`, `RENDER-2`, `RENDER-7` |
| `TR3` | `RENDER-2`, `RENDER-4` |
| `TR4` | `RENDER-2`, `RENDER-3`, `RENDER-5` |
| `TR5` | `RENDER-6`, `RENDER-7` |
| `TR6` | `RENDER-8` |
| `T8.1` | `OPT-1`, `VIEW-4` |
| `T8.2` | `OPT-2` |
| `T8.3` | `OPT-3` |

## 5. Cek Kelengkapan

- [x] Semua requirement PRD memiliki task.
- [x] Semua task memiliki minimal satu requirement.
- [x] Setiap domain memiliki file PRD dan file task.
- [x] Dependency antar-fase tercatat di [`TASKS.md`](TASKS.md#4-dependency-graph).
- [x] Status implementasi diperbarui (Phase 0, track R, Phase 1, Phase 2, Phase 3, Phase 4, Phase 5, Phase 6, Phase 7, dan Phase 8 dievaluasi).
