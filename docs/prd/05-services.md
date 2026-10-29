# PRD — Services

Status: Draft  
Domain: Services  
Requirement: `SVC-*`  
Task: [`../tasks/05-services.md`](../tasks/05-services.md)  
Induk: [`../PRD.md`](../PRD.md)

---

## 1. Tujuan Domain

Membuat service memiliki lifecycle, health, dan shutdown yang seragam. Service tetap berada di luar core dan tidak mengetahui UI/module. Kegagalan satu service tidak boleh menghentikan aplikasi.

## 2. Temuan Saat Ini

- Service di-spawn langsung dari `AppRuntime::run` dengan pola berbeda-beda.
- Tidak ada handle/stop signal yang seragam.
- `MediaService` memakai global mutable `ACTIVE_PLAYER` dan `MEDIA_SENDER`.
- CAVA mengelola child process sendiri tanpa shutdown terpusat.
- Tidak ada health state; error service hanya masuk log.
- `runtime.rs` menangani semua channel service secara manual.

Referensi: `apps/island-app/src/runtime.rs:255-570`, `crates/island-services/src/lib.rs`, `media.rs:12-13`, `cava.rs:19-114`.

## 3. Requirement

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `SVC-1` | Setiap service mengikuti kontrak `Service` dengan `start()` yang mengembalikan `ServiceHandle`. | P1 | `T5.1` |
| `SVC-2` | Service menerima sinyal shutdown. | P1 | `T5.1`, `T5.6` |
| `SVC-3` | Service tidak mengetahui UI, view, atau module. | P1 | `T5.1` |
| `SVC-4` | Service hanya mengirim payload domain atau `DomainEvent`; tidak mengirim aksi UI. | P1 | `T5.1` |
| `SVC-5` | `ServiceSupervisor` mencatat status `Starting`, `Running`, `Degraded`, dan `Stopped`. | P1 | `T5.2` |
| `SVC-6` | Kegagalan satu service tidak menghentikan aplikasi. | P1 | `T5.2` |
| `SVC-7` | State global mutable di service dihapus atau dibatasi pada satu instance eksplisit. | P1 | `T5.4` |
| `SVC-8` | Shutdown menghentikan timer, memberi sinyal ke service, dan join dengan timeout. | P1 | `T5.6` |

## 4. Kontrak

```rust
pub trait Service: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn start(&self, ctx: ServiceContext) -> ServiceHandle;
}

pub struct ServiceHandle {
    pub name: &'static str,
    pub shutdown: ShutdownSignal,
    pub join: JoinHandle<()>,
}
```

Aturan:

- `ServiceContext` membawa sender domain, config read-only, dan shutdown token.
- Service tidak menerima `IslandContext`, `AppState`, atau aksi Slint.
- Start tidak boleh panic; error dikembalikan atau dikirim sebagai health update.
- Setiap service memiliki nama unik untuk log dan health.
- Service yang spawn child process wajib menangani kill saat shutdown.

## 5. Target Lifecycle

```text
ServiceSupervisor::start_all()
  -> service.start(ctx)
  -> status Running / Degraded

Runtime event loop
  -> health update -> log / UI opsional

Shutdown
  -> stop accepting events
  -> timer.cancel_all()
  -> send shutdown signal
  -> join with timeout
  -> persist settings if dirty
```

## 6. Acceptance Criteria

- Semua service dapat start dan stop lewat supervisor.
- Satu service gagal start tidak menghentikan aplikasi.
- Service gagal runtime mengubah health menjadi `Degraded`.
- Tidak ada global mutable di `MediaService`.
- Shutdown tidak meninggalkan child process CAVA/pactl atau task tanpa sinyal.
- Log kegagalan menyebut nama service dan alasan.

## 7. Migrasi Service

Urutan yang disarankan:

1. Clock
2. Battery
3. Media
4. DND
5. Audio
6. Notifications
7. Prayer
8. ScreenSaver
9. CAVA
10. OverlapDetector (platform service bila sesuai)

Satu service per perubahan agar regresi mudah dilacak.

## 8. Test yang Dibutuhkan

| Test | Cakupan |
|---|---|
| Start semua service | `SVC-1`, `SVC-5` |
| Service gagal start → Degraded | `SVC-5`, `SVC-6` |
| Service gagal runtime → health berubah | `SVC-5` |
| Shutdown semua service | `SVC-2`, `SVC-8` |
| Media selection tanpa global | `SVC-7` |
| Child process CAVA berhenti | `SVC-8` |

## 9. Out of Scope

- Service discovery eksternal.
- Restart otomatis dengan backoff kompleks; minimal health state.
- Multi-instance service.
- Mengganti D-Bus library.

## 10. Risiko

| Risiko | Mitigasi |
|---|---|
| Supervisor menambah lapisan yang sulit dites | Kontrak minimal dan test lifecycle |
| Shutdown menggantung karena service tidak responsif | Timeout dan log service yang gagal join |
| Migrasi CAVA meninggalkan child process | Test kill child dan gunakan guard |
| Global media masih dipakai | Migrasi satu per satu; hapus static setelah semua call site pindah |
