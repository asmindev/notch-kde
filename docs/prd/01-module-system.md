# PRD — Module System

Status: Draft  
Domain: Module system  
Requirement: `MOD-*`  
Task: [`../tasks/01-module-system.md`](../tasks/01-module-system.md)  
Induk: [`../PRD.md`](../PRD.md)

---

## 1. Tujuan Domain

Membuat module menjadi unit yang dapat didaftarkan, divalidasi, dan ditampilkan tanpa mengubah `ViewEngine`, platform, atau shell utama. Module tidak boleh memiliki akses langsung ke UI, service, filesystem, atau channel runtime.

## 2. Temuan Saat Ini

- `IslandModule` hanya menyediakan metadata dan `priority()`; tidak ada registry.
- Registrasi module masih dilakukan di `ViewEngine::new`.
- UI memakai angka `active-module` yang dipetakan manual di `view_model.rs`.
- Slot UI untuk tiap module tersebar di tiga view dan tidak divalidasi terhadap module yang terdaftar.
- Module baru membutuhkan perubahan di enum core, implementasi module, engine, view model, dan view Slint.

Referensi: `crates/island-core/src/view_engine.rs:56-64`, `crates/island-ui/src/view_model.rs:98-101`, `crates/island-ui/ui/views/compact-view.slint:6-18`.

## 3. Requirement

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `MOD-1` | Setiap module memiliki `ModuleId` stabil dan unik. | P0 | `T1.1` |
| `MOD-2` | Setiap module memiliki `ModuleDescriptor` berisi id, nama, tier yang didukung, dan default priority. | P0 | `T1.1` |
| `MOD-3` | Module diregistrasikan sekali saat startup melalui `ModuleRegistry`. | P0 | `T1.2` |
| `MOD-4` | Registrasi duplikat menghasilkan error startup yang jelas, bukan overwrite diam-diam. | P0 | `T1.2` |
| `MOD-5` | Module menerima `ModuleContext` read-only; module tidak boleh mengakses service, UI, filesystem, atau channel. | P0 | `T1.1`, `T2.1` |
| `MOD-6` | UI memiliki registry terpisah yang memetakan `ModuleId` ke slot per tier. | P0 | `T1.4` |
| `MOD-7` | Module tanpa slot UI untuk tier tertentu menghasilkan warning dan fallback `System`; aplikasi tidak blank. | P0 | `T1.4` |

## 4. Kontrak

```rust
pub struct ModuleId(pub &'static str);

pub struct ModuleDescriptor {
    pub id: ModuleId,
    pub name: &'static str,
    pub tiers: ModuleTiers,
    pub default_priority: Priority,
}

pub trait IslandModule: Send + Sync {
    fn descriptor(&self) -> ModuleDescriptor;
    fn priority(&self, ctx: &ModuleContext<'_>) -> Priority;
}
```

Aturan kontrak:

- `ModuleId` memakai `&'static str`; tidak perlu alokasi untuk module bawaan.
- Registry menyimpan module dengan urutan registrasi yang deterministik.
- Lookup by ID tidak boleh bergantung pada urutan `Vec` secara implisit.
- `ModuleContext` adalah snapshot read-only, bukan `&AppState`.
- `IslandModule` tidak boleh menyebut tipe Slint atau zbus.

## 5. Alur Penambahan Module

```text
1. Definisikan ModuleId + descriptor
2. Implementasikan IslandModule::priority()
3. Daftarkan di ModuleRegistry
4. Tambahkan slot UI per tier
5. Daftarkan mapping di ModuleUiRegistry
6. Tambahkan test priority + fallback
```

Target: langkah 1–3 hanya menyentuh core module; langkah 4–5 hanya menyentuh UI registry/slot. `ViewEngine` dan shell tidak berubah. Panduan implementasi aktual ada di [`../tasks/01-module-system.md`](../tasks/01-module-system.md#panduan-menambah-module).

## 6. Acceptance Criteria

- `ModuleId` duplikat ditolak dengan error yang menyebut ID tersebut.
- `ViewEngine` menerima registry atau hasil resolver, bukan membuat module sendiri.
- Module dummy dapat ditambahkan dan ditampilkan tanpa mengubah `ViewEngine`.
- Module tanpa slot UI tidak membuat tampilan blank.
- Semua module existing memiliki descriptor dan test priority.
- `cargo test --workspace` tetap lulus.

## 7. Test yang Dibutuhkan

| Test | Cakupan |
|---|---|
| Registry duplicate ID | `MOD-3`, `MOD-4` |
| Registry lookup deterministik | `MOD-3` |
| Descriptor per module | `MOD-2` |
| Priority existing tidak berubah | Regresi `MOD-2` |
| UI binding per tier | `MOD-6` |
| Missing slot fallback | `MOD-7` |

## 8. Out of Scope

- Dynamic plugin loading.
- Module dari crate eksternal saat runtime.
- Hot reload module.
- Dependency injection container.

## 9. Risiko

| Risiko | Mitigasi |
|---|---|
| Migrasi enum `ActiveModule` memecahkan banyak test | Sediakan konversi eksplisit dan pertahankan alias sementara |
| Duplikasi registry core dan UI | Validasi silang saat startup; satu ID canonical |
| Fallback menutupi kesalahan developer | Log warning dengan ID dan tier yang hilang |
