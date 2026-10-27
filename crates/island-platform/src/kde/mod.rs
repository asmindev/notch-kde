use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub screen_index: u32,
}

pub struct KdeManager;

impl KdeManager {
    pub fn new() -> Self {
        Self
    }

    /// Queries the target primary screen geometry (global x, y, logical width, height, screen index)
    pub fn get_primary_screen_geometry() -> ScreenGeometry {
        if let Some(geom) = Self::read_kscreen_doctor() {
            info!(
                "Detected active screen geometry via kscreen-doctor: pos=({},{}) size={}x{} (screen: {})",
                geom.x, geom.y, geom.width, geom.height, geom.screen_index
            );
            return geom;
        }

        if let Some(geom) = Self::read_kwin_output_config() {
            info!(
                "Detected KDE Plasma target screen geometry via config: pos=({},{}) size={}x{} (screen: {})",
                geom.x, geom.y, geom.width, geom.height, geom.screen_index
            );
            return geom;
        }

        // Fallback default for laptop eDP-1
        ScreenGeometry {
            x: 0,
            y: 0,
            width: 1516,
            height: 947,
            screen_index: 0,
        }
    }

    /// Backwards compatibility helper returning (logical width, logical height)
    pub fn get_primary_screen_size() -> (u32, u32) {
        let geom = Self::get_primary_screen_geometry();
        (geom.width, geom.height)
    }

    /// Real-time active output inspection using kscreen-doctor JSON
    fn read_kscreen_doctor() -> Option<ScreenGeometry> {
        let output = std::process::Command::new("kscreen-doctor")
            .arg("-j")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
        let outputs = json.get("outputs")?.as_array()?;

        let target_name = std::env::var("ISLAND_SCREEN_NAME").ok();

        // 1. Try matching target_name (if set by user)
        // 2. Otherwise find "eDP-1" (Laptop built-in display)
        // 3. Otherwise find priority 1 or type 7 (Internal panel)
        // 4. Otherwise first enabled output
        let target_output = outputs
            .iter()
            .find(|o| {
                if let Some(ref name) = target_name {
                    o.get("name").and_then(|n| n.as_str()) == Some(name)
                } else {
                    false
                }
            })
            .or_else(|| {
                outputs.iter().find(|o| {
                    o.get("name").and_then(|n| n.as_str()) == Some("eDP-1")
                        && o.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false)
                })
            })
            .or_else(|| {
                outputs.iter().find(|o| {
                    let enabled = o.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
                    let prio = o.get("priority").and_then(|p| p.as_u64()).unwrap_or(99);
                    enabled && prio == 1
                })
            })
            .or_else(|| {
                outputs.iter().find(|o| {
                    o.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false)
                })
            })?;

        let pos = target_output.get("pos")?;
        let pos_x = pos.get("x")?.as_i64()? as i32;
        let pos_y = pos.get("y")?.as_i64()? as i32;

        let scale = target_output.get("scale").and_then(|s| s.as_f64()).unwrap_or(1.0);
        let size = target_output.get("size")?;
        let width_raw = size.get("width")?.as_f64()?;
        let height_raw = size.get("height")?.as_f64()?;

        let logical_w = (width_raw / scale).round() as u32;
        let logical_h = (height_raw / scale).round() as u32;

        Some(ScreenGeometry {
            x: pos_x,
            y: pos_y,
            width: logical_w,
            height: logical_h,
            screen_index: 0,
        })
    }

    fn read_kwin_output_config() -> Option<ScreenGeometry> {
        let home = std::env::var("HOME").ok()?;
        let path = std::path::Path::new(&home).join(".config/kwinoutputconfig.json");
        let content = std::fs::read_to_string(path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;

        let target_idx: u64 = std::env::var("ISLAND_SCREEN")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0); // Default to Screen 0 (Laptop display eDP-1)

        let configs = json.as_array()?;
        let first_cfg = configs.first()?;

        // 1. Get display mode & scale from data array
        let data = first_cfg.get("data")?.as_array()?;
        let target_data = data.get(target_idx as usize).or_else(|| data.first())?;

        let scale = target_data.get("scale").and_then(|s| s.as_f64()).unwrap_or(1.0);
        let mode = target_data.get("mode");
        let hdisplay = mode
            .and_then(|m| m.get("cvt"))
            .and_then(|c| c.get("hdisplay"))
            .and_then(|h| h.as_u64())
            .or_else(|| mode.and_then(|m| m.get("width")).and_then(|w| w.as_u64()))
            .unwrap_or(2880);

        let vdisplay = mode
            .and_then(|m| m.get("cvt"))
            .and_then(|c| c.get("vdisplay"))
            .and_then(|v| v.as_u64())
            .or_else(|| mode.and_then(|m| m.get("height")).and_then(|h| h.as_u64()))
            .unwrap_or(1800);

        let logical_w = ((hdisplay as f64) / scale).round() as u32;
        let logical_h = ((vdisplay as f64) / scale).round() as u32;

        // 2. Find global x, y position in outputs layouts
        let mut pos_x = 0i32;
        let mut pos_y = 0i32;

        if let Some(output_groups) = first_cfg.get("outputs").and_then(|o| o.as_array()) {
            let mut best_group = None;
            let mut max_outs = 0;

            for group in output_groups {
                let lid_closed = group.get("lidClosed").and_then(|l| l.as_bool()).unwrap_or(false);
                if lid_closed {
                    continue;
                }
                if let Some(outs) = group.get("outputs").and_then(|o| o.as_array()) {
                    if outs.len() >= max_outs {
                        max_outs = outs.len();
                        best_group = Some(outs);
                    }
                }
            }

            if let Some(outs) = best_group {
                for out in outs {
                    let out_idx = out.get("outputIndex").and_then(|i| i.as_u64()).unwrap_or(0);
                    if out_idx == target_idx {
                        if let Some(p) = out.get("position") {
                            pos_x = p.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            pos_y = p.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        }
                        break;
                    }
                }
            }
        }

        Some(ScreenGeometry {
            x: pos_x,
            y: pos_y,
            width: logical_w,
            height: logical_h,
            screen_index: target_idx as u32,
        })
    }
}
