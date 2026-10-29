# PRD — Render & Activity Policy

Status: Draft  
Domain: Render & activity  
Requirement: `RENDER-*`  
Task: [`../tasks/09-render-policy.md`](../tasks/09-render-policy.md)  
Induk: [`../PRD.md`](../PRD.md)

---

## 1. Tujuan Domain

Menentukan secara eksplisit kapan aplikasi boleh menggambar, menjalankan animasi, memperbarui spectrum, dan melakukan render 3D. Tujuannya mengurangi pekerjaan background tanpa mengubah perilaku visual yang terlihat.

Domain ini menjawab pertanyaan:

- Apakah Compact perlu dirender saat view aktif adalah Wide/Expand?
- Apakah aplikasi tetap merender saat `is-visible-state = false`?
- Apakah pet 3D dan timer Slint berhenti saat tidak terlihat?
- Kapan CAVA/spectrum boleh memicu update UI?
- Apakah transisi boleh merender dua view sekaligus?

## 2. Temuan Saat Ini

- Compact, Wide, dan Expand selalu diinstansiasi di `main.slint`; komentar “Zero-Instantiation” tidak akurat.
- View hidden memakai `opacity: 0` dan `visible: opacity > 0.01`, sehingga setelah crossfade tidak dilukis. Namun komponen dan timer di dalamnya tetap hidup.
- Slot Wide/Expand memakai `visible:` dan selalu dibuat; slot Compact memakai `if` berdasarkan `active-module`, tetapi tetap dibuat meski Compact tidak terlihat.
- Pet 3D berjalan ~30 FPS (`FRAME_INTERVAL = 33ms`), `tick()` selalu mengembalikan `dirty: true`, dan timer utama selalu `request_redraw()` tanpa cek visibility.
- Timer settings sudah mengecek `is_visible()`; timer utama belum.
- Beberapa timer Slint berjalan tanpa syarat: `breath-timer`, prayer pulse `root.is-hovered || true`, alert/charging pulse, dan star field.
- `is-visible-state = false` hanya menggeser notch ke `y: -160px`; surface 480×208 dan input strip 28px tetap ada.
- Sync UI penuh terjadi setiap perubahan state, termasuk update spectrum CAVA.

Referensi: `crates/island-ui/ui/main.slint:51-91`, `wide-view.slint:80-101`, `expand-view.slint:107-122`, `island-shell.slint:122-132`, `apps/island-app/src/pet3d.rs:14`, `pet3d.rs:376-381`, `pet3d.rs:522-535`, `apps/island-app/src/input_region.rs:25-52`, `crates/island-ui/src/view_model.rs:60-75`, `view_model.rs:168-224`, `prayer-expand-slot.slint:22-25`, `blobatar-pet.slint:216-222`.

## 3. Requirement

| ID | Requirement | Prioritas | Task |
|---|---|---|---|
| `RENDER-1` | `RenderPolicy` diturunkan dari state dan menentukan `main_visible`, `pet_3d`, `spectrum`, dan `animations`. | P1 | `TR1` |
| `RENDER-2` | Saat hidden, aplikasi tidak melakukan redraw kontinu, tidak menjalankan tick pet 3D, dan tidak menjalankan timer animasi. | P0 | `TR2`, `TR3`, `TR4` |
| `RENDER-3` | Di luar transisi, hanya view tier aktif yang dilukis; transisi boleh melukis view lama dan baru selama durasi crossfade. | P1 | `TR4` |
| `RENDER-4` | Pet 3D hanya di-tick dan dirender jika terlihat dan slot aktif memakai pet; `dirty` tidak boleh selalu `true`. | P1 | `TR3` |
| `RENDER-5` | Timer Slint hanya berjalan saat komponen aktif/terlihat; pola `running: ... \|\| true` dilarang. | P1 | `TR3` |
| `RENDER-6` | Update spectrum/CAVA ke UI hanya dilakukan jika ada slot aktif yang memakai spectrum. | P2 | `TR5` |
| `RENDER-7` | Saat hidden, surface dan input strip boleh tetap ada untuk reveal, tetapi tidak boleh memicu render work. | P1 | `TR2`, `TR5` |
| `RENDER-8` | Activity counters tersedia dan hasil pengukuran didokumentasikan. | P2 | `TR6` |

## 4. Keputusan Render

| State | View yang dilukis | Pet 3D | Spectrum UI | Timer animasi |
|---|---|---|---|---|
| Compact visible | Compact | Jika slot memakai pet | Jika media playing | Hanya timer compact |
| Wide visible | Wide | Jika slot memakai pet | Jika media playing | Hanya timer wide |
| Expand visible | Expand | Jika slot memakai pet | Jika media playing | Hanya timer expand |
| Transisi | View lama + baru | Ikuti view yang terlihat | Ikuti kebutuhan | Hanya selama transisi |
| Hidden | Tidak ada | Berhenti | Berhenti | Berhenti |
| Compact tidak aktif | Tidak dilukis | Berhenti jika tidak ada pet lain | Berhenti | Berhenti |

Aturan tambahan:

- Crossfade 150–260 ms boleh merender dua view; ini bukan kebocoran.
- `visible: false` menghentikan paint, tetapi bukan timer. Karena itu timer harus digate terpisah.
- `opacity: 0` saja tidak cukup; tetap gunakan `visible` setelah animasi selesai.
- Jangan destroy seluruh view di tengah animasi. Pertahankan wrapper, gate heavy children.
- Hidden bukan berarti window harus di-hide total; input strip boleh dipertahankan untuk reveal. Yang penting tidak ada render loop.

## 5. Kontrak

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderPolicy {
    pub main_visible: bool,
    pub pet_3d: bool,
    pub spectrum: bool,
    pub animations: bool,
}

pub fn render_policy(state: &AppState) -> RenderPolicy;
```

Aturan kontrak:

- `RenderPolicy` adalah logika murni dan dapat dites tanpa Slint/KDE.
- `island-ui` hanya memetakan flag ke properti context; tidak menghitung ulang aturan.
- `island-app` memakai `pet_3d` dan `main_visible` untuk mengontrol timer/renderer.
- `spectrum` mengontrol update model/visualizer, bukan lifecycle service CAVA.
- `animations` mengontrol timer dekoratif, bukan transisi wajib.
- Jangan menambah `RenderPolicy` per komponen; satu policy per state.

## 6. Acceptance Criteria

- Saat hidden lebih dari durasi crossfade, tidak ada `request_redraw` periodik dari timer pet.
- Saat hidden, `Pet3dInner::tick()` tidak dipanggil oleh timer periodik.
- Saat Compact tidak aktif, timer compact (pet, pulse, CAVA) tidak berjalan.
- Saat mode Wide/Expand, Compact tidak dilukis setelah crossfade selesai.
- Tidak ada timer dengan `running: ... || true`.
- Spectrum tidak memperbarui model UI saat tidak ada slot yang memakai spectrum.
- Perilaku visual sama pada smoke test Compact/Wide/Expand/hidden.
- Ada catatan hasil pengukuran baseline dan sesudah perubahan.

## 7. Test yang Dibutuhkan

| Test | Cakupan |
|---|---|
| Policy matrix visible/hidden | `RENDER-1` |
| Hidden tidak menaikkan redraw counter | `RENDER-2`, `RENDER-7` |
| Pet idle dirty hanya saat berubah | `RENDER-4` |
| Timer hidden tidak jalan | `RENDER-5` |
| Spectrum gating | `RENDER-6` |
| Crossfade tetap mengizinkan dua view | `RENDER-3` |
| Measurement record | `RENDER-8` |

## 8. Out of Scope

- Menghapus surface/window saat hidden.
- Mengganti renderer Slint atau WGPU.
- Menurunkan FPS pet saat visible tanpa data pengukuran.
- Dynamic instantiation seluruh view per frame.
- Visual regression automation.

## 9. Risiko

| Risiko | Mitigasi |
|---|---|
| Gate terlalu agresif membuat animasi terpotong | Pisahkan `animations` dari `main_visible`; test transisi |
| Hidden input strip masih memicu event | Dokumentasikan tradeoff; ukur event rate |
| Policy dihitung ulang setiap frame | Hitung saat state berubah, simpan di snapshot |
| Timer berhenti saat seharusnya jalan | Test matrix per tier dan media state |
| Pet 3D berhenti saat terlihat | Test `pet_3d` per mode/module/music-visual-mode |
