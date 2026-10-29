# PRD — Configuration

Status: Draft  
Domain: Configuration  
Requirement: `CFG-*`  
Task: [`../tasks/06-configuration.md`](../tasks/06-configuration.md)  
Induk: [`../PRD.md`](../PRD.md)

---

## 1. Tujuan Domain

Memisahkan konfigurasi deployment/internal dari preferensi pengguna, memusatkan dimensi window, dan menghilangkan panic/konfigurasi tersebar. Semua konfigurasi harus tervalidasi dan memiliki fallback.

## 2. Temuan Saat Ini

- `PersistedSettings` mencampur preferensi user dengan beberapa nilai internal.
- `ISLAND_DODGE` dibaca di `island-core/src/transition_policy.rs`.
- Dimensi window/notch tersebar di `runtime.rs`, `bootstrap.rs`, `input_region.rs`, dan `metrics.slint`.
- Path settings dan CAVA dikelola di tempat berbeda.
- `KdeManager::get_primary_screen_geometry()` memakai `.expect(...)`.
- `settings.version` ada, tetapi migration belum menjadi kontrak eksplisit.

Referensi: `crates/island-core/src/settings.rs:6-178`, `crates/island-core/src/transition_policy.rs:25`, `apps/island-app/src/bootstrap.rs:19-26`, `runtime.rs:17-37`, `input_region.rs:17-53`, `crates/island-platform/src/kde/mod.rs:27-29`.

## 3. Requirement

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `CFG-1` | `AppConfig` terpisah dari `PersistedSettings`. | P1 | `T6.1` |
| `CFG-2` | `AppConfig` memuat window spec, enable dodge, path, service enable, dan path CAVA. | P1 | `T6.1`, `T6.2` |
| `CFG-3` | `PersistedSettings` tetap memuat preferensi pengguna: avatar, behavior, shadow, prayer, media visual. | P1 | `T6.1` |
| `CFG-4` | Environment variable developer tidak dibaca di `island-core`. | P1 | `T6.3` |
| `CFG-5` | Konfigurasi divalidasi saat load; jika gagal, gunakan default dan catat warning. | P1 | `T6.2` |
| `CFG-6` | `settings.version` dipakai untuk migration; versi lama tidak boleh membuat aplikasi gagal. | P1 | `T6.5` |
| `CFG-7` | Dimensi window, notch, dan input region berasal dari satu sumber atau `WindowSpec`. | P1 | `T6.4` |
| `CFG-8` | Perubahan preferensi saat runtime tetap melalui event dan `Effect::PersistSettings`. | P1 | `T6.2`, `T6.5` |

## 4. Pemisahan Konfigurasi

```text
AppConfig (deployment/internal)
├── window: WindowSpec
├── dodge_enabled: bool
├── config_dir: PathBuf
├── cava: CavaConfig
└── services: ServiceToggles

PersistedSettings (preferensi user)
├── version
├── avatar
├── behavior
├── shadow
├── prayer
└── media_visual
```

Aturan:

- `AppConfig` tidak disimpan ke `settings.json` user.
- `PersistedSettings` tidak memuat path tool atau flag developer.
- `AppConfig` dimuat sekali saat bootstrap dan diteruskan read-only.
- Perubahan settings user tidak boleh mengubah `AppConfig` secara langsung.

## 5. WindowSpec

`WindowSpec` minimal memuat:

- ukuran window utama;
- compact width/height;
- alert width/height;
- wide width/height;
- expand width/height;
- ear/touch height;
- margin/inset yang memengaruhi hit region.

`WindowSpec` harus menjadi sumber untuk:

- ukuran window Slint;
- `input_region.rs`;
- `bootstrap.rs` window rules;
- nilai `ThemeMetrics` yang terkait.

## 6. Acceptance Criteria

- `ISLAND_DODGE` tidak dibaca di `island-core`.
- Config malformed tidak membuat panic.
- Tidak ada panic saat `kscreen-doctor` tidak tersedia.
- Settings lama tetap dapat dimuat.
- Dimensi notch pada input region dan tampilan berasal dari sumber yang sama.
- `cargo test --workspace` tetap lulus.

## 7. Test yang Dibutuhkan

| Test | Cakupan |
|---|---|
| Default config | `CFG-1`, `CFG-2` |
| Partial JSON | `CFG-5` |
| Malformed JSON | `CFG-5` |
| Out-of-range values | `CFG-5` |
| Migration versi lama | `CFG-6` |
| Env override hanya di layer yang diizinkan | `CFG-4` |
| Geometry fallback | `CFG-7` |

## 8. Out of Scope

- UI editor untuk `AppConfig`.
- Remote config.
- Sinkronisasi settings antar-perangkat.
- Format config selain JSON.

## 9. Risiko

| Risiko | Mitigasi |
|---|---|
| Settings user lama tidak kompatibel | Migration test dan default backward-compatible |
| Dua sumber ukuran window | Jadikan `WindowSpec` satu-satunya sumber; audit hardcode |
| Config loader membaca env di core | Layer config hanya di app; core menerima nilai sudah jadi |
| CAVA config pindah dan merusak path | Pertahankan default path dan fallback |
