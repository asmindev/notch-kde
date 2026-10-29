# PRD — Optional Cleanup

Status: Draft  
Domain: Optional cleanup  
Requirement: `OPT-*`  
Task: [`../tasks/08-optional-cleanup.md`](../tasks/08-optional-cleanup.md)  
Induk: [`../PRD.md`](../PRD.md)

---

## 1. Tujuan Domain

Menampung perbaikan yang bermanfaat tetapi tidak diperlukan untuk modularisasi inti. Domain ini dikerjakan hanya jika kompleksitas module/service sudah meningkat atau ada kebutuhan nyata.

## 2. Requirement

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `OPT-1` | Pisahkan `SettingsContext` dari `IslandContext` bila settings window tidak lagi membutuhkan state domain. | P2 | `T8.1` |
| `OPT-2` | Pisahkan `UiAction` dan `DomainEvent` bila service/module bertambah banyak. | P2 | `T8.2` |
| `OPT-3` | Evaluasi dan promosikan `Welcome` menjadi module/overlay bila kondisi tersebar mulai sulit dilacak. | P2 | `T8.3` |

## 3. Kondisi Pemicu

Kerjakan domain ini hanya jika salah satu kondisi berikut terpenuhi:

- Settings window membawa lebih dari 5 properti domain yang tidak digunakan.
- Service mulai mengirim aksi yang hanya relevan untuk UI.
- `is_welcome` muncul di lebih dari 3 tempat dengan logika berbeda.
- Penambahan module keempat/kelima membutuhkan perubahan besar di luar registry.

### 3.1 Hasil Evaluasi

| Task | Keputusan | Alasan |
|---|---|---|
| `T8.1` | Diterapkan sebagai pemisahan terarah | Settings memakai settings, jadwal prayer, app version, dan preview pet. `sync_prayer_settings` hanya mengisi state prayer yang dipakai settings (tanpa panel/alert/orbit/sky); global terpisah tidak dibuat karena properti settings dipakai bersama main UI dan komponen. |
| `T8.2` | Tidak dikerjakan | `DomainEvent` sudah terpisah di `island-services`; service hanya mengirim `DomainEvent` dan tidak mereferensikan `AppEvent`/`IslandContext`. Bridge app yang memetakan ke `AppEvent` sudah menjadi batas UI. |
| `T8.3` | Dokumentasi keputusan | `Welcome` tetap `PresentationState`. Ini overlay Wide transien yang memengaruhi shell, slot, dan animasi pet; module baru akan menduplikasi `RenderPolicy` tanpa manfaat. |

Hasil: satu perubahan kode terarah pada sinkronisasi settings; `T8.2` dan promosi Welcome tidak dikerjakan karena tidak ada manfaat bersih.

## 4. Acceptance Criteria

- `OPT-1`: Settings window hanya menerima state yang dibutuhkan; tidak ada regresi settings.
- `OPT-2`: Service tidak mengetahui `UiAction`; test domain event lulus.
- `OPT-3`: Keputusan welcome terdokumentasi; tidak ada kondisi welcome yang tersebar tanpa alasan.

## 5. Out of Scope

- Rewrite total context Slint.
- Memindahkan semua event ke event bus baru.
- Menambah module baru hanya untuk membuktikan arsitektur.

## 6. Risiko

| Risiko | Mitigasi |
|---|---|
| Context split memecahkan binding Slint | Lakukan setelah Phase 7; smoke test settings |
| Event split menambah boilerplate | Hanya jika service benar-benar mengirim aksi UI |
| Welcome module menambah state machine baru | Pastikan ada manfaat jelas; jika tidak, pertahankan state presentasi |
