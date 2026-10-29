# PRD — Timer & Effect

Status: Draft  
Domain: Timer & effect  
Requirement: `TIMER-*`  
Task: [`../tasks/04-timers-effects.md`](../tasks/04-timers-effects.md)  
Induk: [`../PRD.md`](../PRD.md)

---

## 1. Tujuan Domain

Membuat timer memiliki identitas bertipe, dapat dibatalkan, dan dieksekusi melalui satu executor. Menghilangkan mega-switch timer di `runtime.rs` dan mengurangi risiko timer stale.

Domain ini mengatur timer domain/runtime. Gating timer animasi Slint berdasarkan visibility ada di [`10-render-policy.md`](10-render-policy.md).

## 2. Temuan Saat Ini

- `Effect::ScheduleTimer` memakai `TimerId` bertipe setelah `T4.1`.
- `runtime.rs` telah memakai `EffectExecutor` dan `TimerScheduler`; cabang besar per ID timer dihapus setelah `T4.3`.
- `CancelTimer` tidak lagi menjadi effect; pembatalan ditangani `TimerScheduler` melalui `cancel`/`cancel_all` setelah `T4.2`.
- Setiap timer memakai pola spawn/sleep/send yang serupa.
- Generation check sudah ada, tetapi belum terpusat.

Referensi: `crates/island-core/src/events.rs:99`, `crates/island-core/src/reducer.rs`, `apps/island-app/src/runtime.rs:599-679`.

## 3. Requirement

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `TIMER-1` | Timer memakai `TimerId` bertipe, bukan string bebas. | P1 | `T4.1` |
| `TIMER-2` | `TimerScheduler` menyimpan handle per timer dan mendukung pembatalan. | P1 | `T4.2` |
| `TIMER-3` | Generation check tetap dipakai untuk mencegah event timer stale. | P1 | `T4.2` |
| `TIMER-4` | `EffectExecutor` dipisahkan dari event loop utama. | P1 | `T4.3` |
| `TIMER-5` | Tidak ada duplikasi pola spawn/sleep/send untuk setiap timer. | P1 | `T4.3` |

## 4. Kontrak

```rust
pub enum TimerId {
    Collapse,
    HoverHide,
    Welcome,
    PrayerRefresh,
    MediaPoll,
    // ...
}

pub struct TimerScheduler { /* handle per TimerId */ }

impl TimerScheduler {
    pub fn schedule(&mut self, id: TimerId, delay: Duration, generation: u64);
    pub fn cancel(&mut self, id: TimerId);
    pub fn cancel_all(&mut self);
}
```

Aturan:

- `TimerId` adalah enum di core; tidak boleh berupa string.
- Scheduling ulang ID yang sama membatalkan timer sebelumnya.
- Event timer membawa ID dan generation.
- Reducer mengabaikan event dengan generation lama.
- Scheduler tidak boleh mengubah state domain; ia hanya mengirim event.

## 5. Target Alur

```text
Reducer -> Effect::ScheduleTimer { id, delay, generation }
Runtime -> EffectExecutor -> TimerScheduler
TimerScheduler -> AppEvent::TimerFired { id, generation }
Reducer -> validasi generation -> transisi
```

## 6. Acceptance Criteria

- Tidak ada string timer di reducer.
- Compiler menangkap timer ID yang tidak valid.
- Schedule baru membatalkan timer lama untuk ID yang sama.
- Cancel bekerja dan tidak meninggalkan task.
- Shutdown membatalkan semua timer.
- Test cancel dan generation lulus.

## 7. Test yang Dibutuhkan

| Test | Cakupan |
|---|---|
| Schedule lalu fire | `TIMER-1`, `TIMER-2` |
| Schedule ulang membatalkan timer lama | `TIMER-2` |
| Cancel | `TIMER-2` |
| Generation stale diabaikan | `TIMER-3` |
| Shutdown membatalkan semua | `TIMER-2`, `T5.6` |

## 8. Out of Scope

- Timer berbasis cron.
- Timer persisten antar-restart.
- Mengganti runtime async yang sudah dipakai.

## 9. Risiko

| Risiko | Mitigasi |
|---|---|
| Perubahan timing animasi overlap | Pertahankan durasi existing; test sequencing |
| Deadlock saat cancel dan fire bersamaan | Handle disimpan di satu owner; kirim event, jangan join di critical path |
| `TimerId` terlalu banyak | Kelompokkan ID yang benar-benar berbeda; jangan membuat ID generik |
