# PRD — Content Resolution

Status: Draft  
Domain: Content resolution  
Requirement: `CONTENT-*`  
Task: [`../tasks/02-content-resolution.md`](../tasks/02-content-resolution.md)  
Induk: [`../PRD.md`](../PRD.md)

---

## 1. Tujuan Domain

Menetapkan satu sumber kebenaran untuk menentukan module/konten yang tampil pada setiap tier view. Slint hanya merender hasil keputusan; Slint tidak boleh menentukan module berdasarkan kondisi bisnis.

## 2. Temuan Saat Ini

- Engine memilih module berdasarkan priority, tetapi reducer juga menulis `active_module` secara langsung.
- `WideView` dan `ExpandView` memiliki kondisi `show-system`, `show-media`, dan `show-prayer` yang dapat berbeda dari hasil engine.
- Media paused dapat dipilih sebagai module Media oleh engine, tetapi WideView merender System.
- Source selector dan prayer panel adalah override, tetapi tidak memiliki representasi alasan keputusan.
- Tidak ada test matrix module × tier × override.

Referensi: `crates/island-core/src/view_engine.rs:88-139`, `crates/island-core/src/reducer.rs:288-293`, `crates/island-ui/ui/views/wide-view.slint:15-31`, `expand-view.slint:15-20`, `compact-view.slint:6-18`.

## 3. Requirement

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `CONTENT-1` | Hanya `ContentResolver` yang menentukan module aktif. | P0 | `T2.2`, `T2.4` |
| `CONTENT-2` | Resolver memfilter module berdasarkan tier yang didukung view aktif. | P0 | `T2.2` |
| `CONTENT-3` | Resolver memilih priority tertinggi; tie-break memakai urutan registrasi yang deterministik. | P0 | `T2.2` |
| `CONTENT-4` | Jika semua module `INACTIVE`, resolver memilih `System`. | P0 | `T2.2` |
| `CONTENT-5` | Source selector dan prayer panel diperlakukan sebagai override eksplisit dengan alasan keputusan. | P0 | `T2.2` |
| `CONTENT-6` | View Slint tidak boleh memiliki aturan bisnis `show-system`/`show-media`/`show-prayer`; view hanya merender hasil resolver. | P0 | `T2.3` |
| `CONTENT-7` | Hasil resolver menyertakan `ContentReason` untuk logging dan test. | P0 | `T2.1`, `T2.2` |

## 4. Semantik Resolusi

Urutan evaluasi yang ditetapkan:

```text
1. Tentukan tier dari ViewEngine.
2. Filter module yang mendukung tier tersebut.
3. Terapkan override eksplisit:
   - prayer panel terbuka
   - source selector terbuka
4. Hitung priority masing-masing module.
5. Pilih priority tertinggi.
6. Tie-break: urutan registrasi.
7. Jika tidak ada yang aktif: System.
```

Aturan tambahan:

- Override tetap harus menghormati tier. Jika module override tidak mendukung tier aktif, gunakan fallback yang terdefinisi dan catat `ContentReason`.
- `active_module` tidak boleh ditulis langsung oleh reducer. Perubahan state yang memengaruhi konten harus memicu resolve ulang.
- `ContentReason` minimal: `Priority`, `UserPanel`, `UserSelector`, `Fallback`, `UnsupportedTier`.

## 5. Kontrak

```rust
pub enum ContentReason {
    Priority,
    UserPanel,
    UserSelector,
    Fallback,
    UnsupportedTier,
}

pub struct ResolvedContent {
    pub module: ModuleId,
    pub priority: Priority,
    pub reason: ContentReason,
}

pub fn resolve(
    registry: &ModuleRegistry,
    ctx: &ModuleContext<'_>,
    tier: ViewTier,
) -> ResolvedContent;
```

`ModuleContext` harus berisi data read-only yang dibutuhkan module, misalnya media snapshot, alert state, prayer state, dan override state. Jangan meneruskan `AppState` utuh.

## 6. Acceptance Criteria

- Tidak ada kondisi `active-module == ...` yang menentukan konten di view; view hanya memakai hasil resolver.
- Media paused, prayer panel, source selector, dan alert memiliki test eksplisit.
- Semua penulisan module aktif berasal dari resolver.
- `ContentReason` tercatat untuk kasus override dan fallback.
- Perilaku visual existing setara pada smoke test.
- `cargo test --workspace` tetap lulus.

## 7. Test Matrix Minimum

| Skenario | Compact | Wide | Expand |
|---|---|---|---|
| Media playing | Ya | Ya | Ya |
| Media paused dengan judul | Ya | Ya | Ya |
| Media idle | System | System | System |
| Prayer panel terbuka | Ya | Ya | Ya |
| Source selector terbuka | Ya | Ya | Ya |
| Alert charging | Ya | Ya | Ya |
| Module tidak mendukung tier | Fallback | Fallback | Fallback |

Setiap test harus memeriksa module hasil dan `ContentReason`.

## 8. Out of Scope

- Membuat editor urutan module di UI.
- Menyimpan preferensi urutan module per pengguna.
- Animasi transisi antar-konten; animasi tetap tanggung jawab view.

## 9. Risiko

| Risiko | Mitigasi |
|---|---|
| Resolver mengubah perilaku media paused | Characterization test sebelum refactor |
| Override prayer/source bertabrakan | Definisikan urutan override dan test keduanya |
| View masih menyimpan aturan tersembunyi | Audit seluruh properti `show-*` dan `active-module` |
