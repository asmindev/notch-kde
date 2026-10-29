# Tasks — Timer & Effect

Fase: 4  
PRD: [`../prd/04-timers-effects.md`](../prd/04-timers-effects.md)  
Index: [`../TASKS.md`](../TASKS.md)

---

## Trace

| Requirement | Task |
|---|---|
| `TIMER-1` | `T4.1` |
| `TIMER-2` | `T4.2` |
| `TIMER-3` | `T4.2` |
| `TIMER-4` | `T4.3` |
| `TIMER-5` | `T4.3` |
| `TEST-4` | `T4.4` |

---

## T4.1 — Definisikan `TimerId`

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T3.2`
- **Trace:** `TIMER-1`
- **Referensi:** `events.rs:99`, `runtime.rs:599-679`, `reducer.rs`.
- **Langkah:**
  1. Buat enum `TimerId` untuk semua timer yang ada.
  2. Ganti `id: &'static str` pada `Effect::ScheduleTimer`.
  3. Ganti seluruh string di reducer.
- **Acceptance:**
  - Tidak ada string timer di reducer.
  - Compiler menangkap timer yang tidak valid.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `TimerId` mencakup seluruh timer reducer; `Effect::ScheduleTimer.id` dan seluruh call site test memakai enum. `rg 'id: "' crates/island-core/src/reducer.rs` tidak menemukan string timer.

## T4.2 — Implementasikan `TimerScheduler`

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T4.1`
- **Trace:** `TIMER-2`, `TIMER-3`
- **Langkah:**
  1. Simpan handle per `TimerId`.
  2. Implementasikan `schedule`, `cancel`, dan `cancel_all`.
  3. Pertahankan generation check.
  4. Pastikan timer lama dibatalkan saat timer baru dijadwalkan untuk ID yang sama.
- **Acceptance:**
  - Test cancel dan generation lulus.
  - Tidak ada timer stale yang mengubah state.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `runtime/timers.rs` menyimpan satu handle per `TimerId`; `schedule` membatalkan timer lama, `cancel`/`cancel_all`/`shutdown` menghentikan task, dan `Drop` membersihkan semua. Scheduler mengirim `AppEvent::TimerFired { id, generation }`; reducer memvalidasi generation per timer. Test schedule, reschedule, cancel, dan shutdown lulus.

## T4.3 — Ekstrak `EffectExecutor`

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T4.2`
- **Trace:** `TIMER-4`, `TIMER-5`
- **Langkah:**
  1. Pindahkan eksekusi `Effect` dari event loop ke modul `runtime/effects.rs`.
  2. Hilangkan mega-switch.
  3. Pastikan executor tidak mengubah state langsung.
- **Acceptance:**
  - Event loop utama lebih pendek dan fokus pada dispatch.
  - Test effect per varian.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `runtime/effects.rs` mengeksekusi seluruh varian `Effect`; event loop hanya memanggil `execute`/`shutdown`. Executor tidak memutasi `AppState` (hanya membaca untuk persist settings). Test memastikan seluruh `TimerId` diteruskan ke scheduler beserta generation.

## T4.4 — Test lifecycle timer

- **Status:** `[ ]`
- **Prioritas:** P1
- **Dependency:** `T4.2`
- **Trace:** `TEST-4`
- **Langkah:**
  1. Test schedule → fire.
  2. Test schedule baru membatalkan timer lama.
  3. Test cancel.
  4. Test shutdown membatalkan semua timer.
- **Acceptance:**
  - Perilaku timer deterministik di test.
- **Risiko:** Rendah.
