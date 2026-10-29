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

- **Status:** `[ ]`
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

## T8.2 — Pisahkan `UiAction` dan `DomainEvent`

- **Status:** `[ ]`
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

## T8.3 — Promosikan Welcome menjadi module/overlay

- **Status:** `[ ]`
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
