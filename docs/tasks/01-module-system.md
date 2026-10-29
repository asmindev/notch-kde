# Tasks — Module System

Fase: 1  
PRD: [`../prd/01-module-system.md`](../prd/01-module-system.md)  
Index: [`../TASKS.md`](../TASKS.md)

---

## Trace

| Requirement | Task |
|---|---|
| `MOD-1` | `T1.1` |
| `MOD-2` | `T1.1`, `T1.3` |
| `MOD-3` | `T1.2` |
| `MOD-4` | `T1.2` |
| `MOD-5` | `T1.1`, `T2.1` |
| `MOD-6` | `T1.4` |
| `MOD-7` | `T1.4` |

---

## T1.1 — Tambah `ModuleId` dan `ModuleDescriptor`

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** `T0.4`
- **Trace:** `MOD-1`, `MOD-2`, `MOD-5`
- **Referensi:** `crates/island-core/src/view_engine.rs:56-64`, `crates/island-core/src/models.rs:14-21`.
- **Langkah:**
  1. Buat `ModuleId(pub &'static str)`.
  2. Buat `ModuleDescriptor` berisi id, name, tiers, default priority.
  3. Tambahkan konstanta ID untuk System, Media, Alert, Prayer.
  4. Pertahankan `ActiveModule` untuk kompatibilitas sementara; sediakan konversi dua arah yang eksplisit.
- **Acceptance:**
  - `ModuleId` dapat dipakai sebagai key lookup.
  - Tidak ada `String` allocation untuk ID statis.
- **Risiko:** Rendah–sedang.
- **Catatan verifikasi:** `ModuleId`, konstanta `SYSTEM`/`MEDIA`/`ALERT`/`PRAYER`, `ModuleDescriptor`, dan `UnknownModuleId` ditambahkan di `crates/island-core/src/module.rs`. Konversi eksplisit `ActiveModule -> ModuleId` serta `TryFrom<ModuleId>` tersedia; `ActiveModule` tetap dipertahankan untuk event dan kompatibilitas test. Test round-trip dan penolakan ID unknown lulus.

## T1.2 — Tambah `ModuleRegistry`

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** `T1.1`
- **Trace:** `MOD-3`, `MOD-4`
- **Langkah:**
  1. Implementasikan registry dengan `Vec<Arc<dyn IslandModule>>`.
  2. Tolak duplikasi ID dengan error yang jelas.
  3. Sediakan lookup berdasarkan `ModuleId` dan iterasi deterministik.
  4. Pindahkan registrasi default dari `ViewEngine::new` ke registry.
- **Acceptance:**
  - `ViewEngine` menerima registry atau hasil resolver, bukan membuat module sendiri.
  - Test duplikasi ID lulus.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `ModuleRegistry` menyimpan `Vec<Arc<dyn IslandModule>>`, menolak ID duplikat lewat `DuplicateModuleId`, menyediakan `get`/`iter` deterministik, dan memindahkan registrasi default dari `ViewEngine::new`. `ViewEngine::with_registry` tersedia; `register_module` mengembalikan error duplikasi. Test duplikasi dan lookup lulus.

## T1.3 — Migrasikan empat module ke descriptor

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** `T1.2`
- **Trace:** `MOD-2`
- **Langkah:**
  1. Implementasikan `descriptor()` untuk System, Media, Alert, Prayer.
  2. Pertahankan `priority()` yang ada tanpa perubahan perilaku.
  3. Tambahkan test descriptor dan priority per module.
- **Acceptance:**
  - Perilaku priority identik dengan baseline.
  - `ModuleTiers` berasal dari descriptor.
- **Risiko:** Sedang; jaga test priority.
- **Catatan verifikasi:** Kontrak module sekarang memakai `descriptor()` + `priority()`. System, Media, Alert, dan Prayer memiliki descriptor lengkap; `ModuleTiers` dan `Priority` dipakai dari descriptor. `ViewEngine` dan `AppState` menyimpan `ModuleId` canonical, sementara enum `ActiveModule` dipertahankan untuk event dan kompatibilitas test lewat konversi/`PartialEq`. Test descriptor, priority per module, dan seluruh suite lulus 140 test.

## T1.4 — Buat `ModuleUiRegistry` di `island-ui`

- **Status:** `[x]`
- **Prioritas:** P0
- **Dependency:** `T1.3`
- **Trace:** `MOD-6`, `MOD-7`
- **Langkah:**
  1. Buat mapping `ModuleId` → slot UI per tier.
  2. Validasi saat startup bahwa setiap module core memiliki slot untuk tier yang didukung.
  3. Jika tidak ada slot, log warning dan gunakan fallback System.
  4. Ganti mapping angka `active-module` di `view_model.rs` dengan mapping dari registry.
- **Acceptance:**
  - Mapping ID ke UI hanya ada di satu tempat.
  - Module tanpa slot tidak membuat tampilan blank.
- **Risiko:** Sedang.
- **Catatan verifikasi:** `ModuleUiRegistry` memetakan `ModuleId` ke `ModuleUiSlot` per tier, dengan fallback `System` + warning untuk ID/tier tanpa slot. `ViewModel` mengisi `compact-module`, `wide-module`, dan `expand-module` dari registry; view Slint memakai properti tier masing-masing. Validasi startup mencatat slot yang hilang (saat ini Alert/Expand) tanpa membuat tampilan blank. Test mapping, fallback, dan validasi lulus; total suite 143 test.

## T1.5 — Dokumentasikan kontrak penambahan module

- **Status:** `[x]`
- **Prioritas:** P1
- **Dependency:** `T1.4`
- **Trace:** `MOD-1`–`MOD-7`
- **Langkah:**
  1. Tambahkan contoh module end-to-end di dokumentasi.
  2. Cantumkan file yang wajib dan tidak boleh disentuh.
  3. Pastikan contoh sesuai dengan implementasi aktual setelah refactor.
- **Acceptance:**
  - Developer baru dapat mengikuti contoh tanpa membaca seluruh core.
- **Risiko:** Rendah.
- **Catatan verifikasi:** Panduan end-to-end ditambahkan di bawah. Contoh memakai API aktual (`ModuleId`, `ModuleDescriptor`, `ModuleRegistry`, `ModuleUiRegistry`) dan mencantumkan file wajib serta file yang tidak boleh disentuh. `ModuleContext` masih berupa `SlotContext` read-only; penggantian nama/kontrak final berada di `T2.1`.

---

## Panduan Menambah Module

Contoh: module `calendar` yang hanya tampil di tier Wide.

### 1. Core module

Tambahkan ID dan implementasi module di `crates/island-core/src/module.rs`:

```rust
impl ModuleId {
    pub const CALENDAR: Self = Self("calendar");
}

#[derive(Debug, Default)]
pub struct CalendarModule;

impl IslandModule for CalendarModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId::CALENDAR,
            name: "Calendar",
            tiers: ModuleTiers::WIDE_ONLY,
            default_priority: Priority::NORMAL,
        }
    }

    fn priority(&self, ctx: &SlotContext<'_>) -> Priority {
        if ctx.view == IslandView::Wide {
            Priority::NORMAL
        } else {
            Priority::INACTIVE
        }
    }
}
```

Daftarkan module sekali saat startup. Untuk module bawaan, tambahkan ke
`ModuleRegistry::with_defaults()`; untuk module aplikasi, gunakan registry eksplisit:

```rust
let mut modules = ModuleRegistry::with_defaults();
modules.register(Arc::new(CalendarModule))?;
let engine = ViewEngine::with_registry(modules);
```

ID duplikat menghasilkan `DuplicateModuleId`; jangan menimpa module lama.

### 2. UI slot

1. Buat komponen slot, misalnya
   `crates/island-ui/ui/modules/calendar/calendar-wide-slot.slint`.
2. Tambahkan varian `ModuleUiSlot::Calendar = 4` di
   `crates/island-ui/src/module_ui_registry.rs`.
3. Tambahkan binding per tier di `DEFAULT_BINDINGS`:

```rust
ModuleUiBinding {
    id: ModuleId::CALENDAR,
    slots: ModuleUiSlots {
        compact: None,
        wide: Some(ModuleUiSlot::Calendar),
        expand: None,
    },
},
```

4. Tambahkan komponen ke komposisi tier yang sesuai, misalnya
   `wide-view.slint`, dengan properti tier dari registry:

```slint
if IslandContext.wide-module == 4: CalendarWideSlot { active: root.active; }
```

Jika slot tidak tersedia, registry mengembalikan fallback `System` dan mencatat warning;
aplikasi tidak boleh blank.

### 3. Test minimum

- Descriptor dan priority module baru.
- Registrasi sukses dan penolakan ID duplikat.
- Mapping UI untuk setiap tier yang didukung.
- Fallback `System` untuk tier yang tidak punya slot.
- Validasi startup tidak menghasilkan warning untuk tier yang didukung.

### 4. File yang tidak perlu disentuh

- `crates/island-core/src/view_engine.rs`
- `crates/island-core/src/state.rs`
- `crates/island-core/src/reducer.rs`
- `crates/island-services/*`
- `crates/island-platform/*`
- `crates/island-ui/ui/main.slint` dan shell, kecuali module memperkenalkan tier/animasi baru

Module hanya menerima snapshot read-only. Jangan menambahkan akses service, filesystem,
channel, atau UI ke implementasi `IslandModule`.
