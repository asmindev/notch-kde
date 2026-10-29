# Tasks — Optional Cleanup

Fase: 8  
PRD: [`../prd/09-optional-cleanup.md`](../prd/09-optional-cleanup.md)  
Index: [`../TASKS.md`](../TASKS.md)

---

## Trace

| Requirement | Task |
|---|---|
| `OPT-1` | `T8.1` |
| `OPT-2` | `T8.2` |
| `OPT-3` | `T8.3` |
| `VIEW-4` | `T8.1` |

---

## T8.1 — Pisahkan `SettingsContext` dari `IslandContext`

- **Status:** `[x]`
- **Prioritas:** P2
- **Dependency:** `T7.3`
- **Trace:** `OPT-1`, `VIEW-4`
- **Langkah:**
  1. Definisikan global terpisah untuk settings.
  2. Pindahkan properti/action settings.
  3. Update `ViewModel` dan `SettingsWindow`.
- **Acceptance:**
  - Settings window tidak membawa state domain yang tidak perlu.
  - Tidak ada regresi settings.
- **Risiko:** Sedang.
- **Catatan verifikasi:** Evaluasi menunjukkan global terpisah tidak sepadan: settings window memakai `IslandContext` untuk settings, jadwal prayer, app version, dan preview `PetView`, sementara navigation/time/battery/media tidak disinkronkan ke settings. Implementasi memakai pemisahan terarah: `sync_prayer_settings` hanya mengisi settings/prayer yang dipakai (enabled, kota, reminder, label, jadwal, subtitle) dan tidak mengirim panel/alert/orbit/sky domain ke settings window. Properti settings tetap di `IslandContext` karena dipakai bersama main UI dan komponen.

## T8.2 — Pisahkan `UiAction` dan `DomainEvent`

- **Status:** `[-]`
- **Prioritas:** P2
- **Dependency:** `T5.6`
- **Trace:** `OPT-2`
- **Langkah:**
  1. Definisikan `UiAction`, `DomainEvent`, `TimerEvent`.
  2. Ubah reducer untuk menerima ketiganya.
  3. Service hanya mengirim `DomainEvent`.
- **Acceptance:**
  - Service tidak mengetahui konsep UI.
- **Risiko:** Sedang–tinggi; hanya jika dibutuhkan.
- **Catatan verifikasi:** Evaluasi menunjukkan batas service sudah bersih: `DomainEvent` didefinisikan di `island-services`, `ServiceContext` hanya mengirim `DomainEvent`, dan tidak ada referensi `island_ui`, `AppEvent`, atau `IslandContext` di crate service. Bridge di `apps/island-app/src/runtime.rs` memetakan `DomainEvent` ke `AppEvent`. Tidak ada aksi UI dari service, sehingga `UiAction`/`TimerEvent` tambahan hanya menambah boilerplate.

## T8.3 — Promosikan Welcome menjadi module/overlay

- **Status:** `[x]`
- **Prioritas:** P2
- **Dependency:** `T2.5`
- **Trace:** `OPT-3`
- **Langkah:**
  1. Evaluasi apakah `is_welcome` tetap state presentasi atau module.
  2. Jika module, daftarkan dengan tier Wide dan priority khusus.
  3. Hapus kondisi welcome yang tersebar di view.
- **Acceptance:**
  - Keputusan terdokumentasi dan tidak menambah kompleksitas tanpa manfaat.
- **Risiko:** Sedang.
- **Catatan verifikasi:** Keputusan: `Welcome` tetap state presentasi (`PresentationState`), bukan module. Welcome adalah overlay Wide transien yang mengubah tinggi/radius shell, menyembunyikan slot konten, dan memicu animasi pet; promosi menjadi module akan menduplikasi `RenderPolicy` dan menambah state machine tanpa manfaat. Kondisi `is-welcome` di Slint tetap satu turunan dari state presentasi.
