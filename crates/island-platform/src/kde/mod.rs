use serde_json::Value;
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

impl Default for KdeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KdeManager {
    pub fn new() -> Self {
        Self
    }

    /// Queries the target primary screen geometry (global x, y, logical width, height, screen index)
    pub fn get_primary_screen_geometry() -> ScreenGeometry {
        if let Some(geom) = Self::read_kscreen_doctor() {
            info!(
                "Detected primary screen geometry via kscreen-doctor: pos=({},{}) size={}x{} (screen: {})",
                geom.x, geom.y, geom.width, geom.height, geom.screen_index
            );
            return geom;
        }

        if let Some(geom) = Self::read_kwin_output_config() {
            info!(
                "Detected primary screen geometry via KWin config: pos=({},{}) size={}x{} (screen: {})",
                geom.x, geom.y, geom.width, geom.height, geom.screen_index
            );
            return geom;
        }

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

    fn read_kscreen_doctor() -> Option<ScreenGeometry> {
        let output = std::process::Command::new("kscreen-doctor")
            .arg("-j")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json: Value = serde_json::from_slice(&output.stdout).ok()?;
        let outputs = json.get("outputs")?.as_array()?;
        let target_name = std::env::var("ISLAND_SCREEN_NAME").ok();
        let target_output = select_target_output(outputs, target_name.as_deref())?;
        let name = target_output.get("name")?.as_str()?;

        let pos = target_output.get("pos")?;
        let pos_x = pos.get("x")?.as_i64()? as i32;
        let pos_y = pos.get("y")?.as_i64()? as i32;

        let scale = target_output
            .get("scale")
            .and_then(Value::as_f64)
            .unwrap_or(1.0);
        let size = target_output.get("size")?;
        let width_raw = size.get("width")?.as_f64()?;
        let height_raw = size.get("height")?.as_f64()?;

        Some(ScreenGeometry {
            x: pos_x,
            y: pos_y,
            width: (width_raw / scale).round() as u32,
            height: (height_raw / scale).round() as u32,
            screen_index: read_kwin_screen_index(name).unwrap_or(0),
        })
    }

    fn read_kwin_output_config() -> Option<ScreenGeometry> {
        let home = std::env::var("HOME").ok()?;
        let path = std::path::Path::new(&home).join(".config/kwinoutputconfig.json");
        let content = std::fs::read_to_string(path).ok()?;
        let json: Value = serde_json::from_str(&content).ok()?;
        let groups = json.as_array()?;

        let outputs = group_data(groups, "outputs")?;
        let setups = group_data(groups, "setups")?;
        let target_name = std::env::var("ISLAND_SCREEN_NAME").ok();

        let target_idx = if let Some(name) = target_name.as_deref() {
            outputs.iter().position(|output| {
                output.get("connectorName").and_then(Value::as_str) == Some(name)
            })?
        } else {
            select_config_primary_index(setups, outputs.len())?
        };

        let target_data = outputs.get(target_idx)?;
        let connector = target_data.get("connectorName").and_then(Value::as_str);

        let setup_outputs = setups
            .iter()
            .filter(|setup| {
                !setup
                    .get("lidClosed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .filter_map(|setup| setup.get("outputs").and_then(Value::as_array))
            .filter(|entries| {
                entries.iter().any(|entry| {
                    entry.get("outputIndex").and_then(Value::as_u64) == Some(target_idx as u64)
                })
            })
            .max_by_key(|entries| entries.iter().filter(|entry| entry_enabled(entry)).count())?;

        let target_entry = setup_outputs
            .iter()
            .find(|entry| {
                entry.get("outputIndex").and_then(Value::as_u64) == Some(target_idx as u64)
                    && entry_enabled(entry)
            })
            .or_else(|| {
                setup_outputs
                    .iter()
                    .find(|entry| entry.get("priority").and_then(Value::as_u64) == Some(1))
            })
            .or_else(|| setup_outputs.iter().find(|entry| entry_enabled(entry)))?;

        let pos = target_entry.get("position")?;
        let pos_x = pos.get("x").and_then(Value::as_i64).unwrap_or(0) as i32;
        let pos_y = pos.get("y").and_then(Value::as_i64).unwrap_or(0) as i32;

        let scale = target_data
            .get("scale")
            .and_then(Value::as_f64)
            .unwrap_or(1.0);
        let mode = target_data.get("mode");
        let hdisplay = mode
            .and_then(|mode| mode.get("cvt"))
            .and_then(|cvt| cvt.get("hdisplay"))
            .and_then(Value::as_u64)
            .or_else(|| {
                mode.and_then(|mode| mode.get("width"))
                    .and_then(Value::as_u64)
            })
            .unwrap_or(2880);
        let vdisplay = mode
            .and_then(|mode| mode.get("cvt"))
            .and_then(|cvt| cvt.get("vdisplay"))
            .and_then(Value::as_u64)
            .or_else(|| {
                mode.and_then(|mode| mode.get("height"))
                    .and_then(Value::as_u64)
            })
            .unwrap_or(1800);

        let screen_index = connector
            .and_then(read_kwin_screen_index)
            .unwrap_or(target_idx as u32);

        Some(ScreenGeometry {
            x: pos_x,
            y: pos_y,
            width: ((hdisplay as f64) / scale).round() as u32,
            height: ((vdisplay as f64) / scale).round() as u32,
            screen_index,
        })
    }
}

fn output_enabled(output: &Value) -> bool {
    output
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn select_target_output<'a>(outputs: &'a [Value], target_name: Option<&str>) -> Option<&'a Value> {
    let explicit_output = target_name.and_then(|name| {
        outputs
            .iter()
            .find(|output| output.get("name").and_then(Value::as_str) == Some(name))
    });
    if let Some(output) = explicit_output {
        return Some(output);
    }

    outputs
        .iter()
        .find(|output| {
            output_enabled(output) && output.get("priority").and_then(Value::as_u64) == Some(1)
        })
        .or_else(|| {
            outputs.iter().find(|output| {
                output.get("name").and_then(Value::as_str) == Some("eDP-1")
                    && output_enabled(output)
            })
        })
        .or_else(|| outputs.iter().find(|output| output_enabled(output)))
}

fn group_data<'a>(groups: &'a [Value], name: &str) -> Option<&'a [Value]> {
    groups
        .iter()
        .find(|group| group.get("name").and_then(Value::as_str) == Some(name))?
        .get("data")?
        .as_array()
        .map(Vec::as_slice)
}

fn entry_enabled(entry: &Value) -> bool {
    entry
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn select_config_primary_index(setups: &[Value], output_count: usize) -> Option<usize> {
    let setup = setups
        .iter()
        .filter(|setup| {
            !setup
                .get("lidClosed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|setup| setup.get("outputs").and_then(Value::as_array))
        .max_by_key(|entries| entries.iter().filter(|entry| entry_enabled(entry)).count())?;

    setup
        .iter()
        .find(|entry| {
            entry_enabled(entry) && entry.get("priority").and_then(Value::as_u64) == Some(1)
        })
        .and_then(|entry| entry.get("outputIndex").and_then(Value::as_u64))
        .map(|index| index as usize)
        .filter(|index| *index < output_count)
        .or_else(|| output_count.checked_sub(1).filter(|_| output_count == 1))
        .or(Some(0))
}

fn read_kwin_screen_index(connector: &str) -> Option<u32> {
    for command in ["qdbus6", "qdbus"] {
        let output = match std::process::Command::new(command)
            .args(["org.kde.KWin", "/KWin", "supportInformation"])
            .output()
        {
            Ok(output) if output.status.success() => output,
            _ => continue,
        };

        let text = String::from_utf8_lossy(&output.stdout);
        if let Some(index) = parse_kwin_screen_index(&text, connector) {
            return Some(index);
        }
    }
    None
}

fn parse_kwin_screen_index(text: &str, connector: &str) -> Option<u32> {
    let mut current = None;

    for line in text.lines() {
        let line = line.trim();

        if let Some(number) = line
            .strip_prefix("Screen ")
            .and_then(|rest| rest.strip_suffix(':'))
        {
            current = number.trim().parse::<u32>().ok();
            continue;
        }

        if let Some(rest) = line.strip_prefix("Name:") {
            let Some(current_index) = current else {
                continue;
            };
            if rest.trim() == connector {
                return Some(current_index);
            }
            current = None;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn primary_priority_beats_edp() {
        let outputs = vec![
            json!({"name": "eDP-1", "enabled": true, "priority": 2}),
            json!({"name": "HDMI-A-1", "enabled": true, "priority": 1}),
        ];

        let selected = select_target_output(&outputs, None).unwrap();
        assert_eq!(selected.get("name").unwrap(), "HDMI-A-1");
    }

    #[test]
    fn explicit_override_wins() {
        let outputs = vec![
            json!({"name": "eDP-1", "enabled": true, "priority": 1}),
            json!({"name": "DP-1", "enabled": true, "priority": 2}),
        ];

        let selected = select_target_output(&outputs, Some("DP-1")).unwrap();
        assert_eq!(selected.get("name").unwrap(), "DP-1");
    }

    #[test]
    fn disabled_primary_is_ignored() {
        let outputs = vec![
            json!({"name": "HDMI-A-1", "enabled": false, "priority": 1}),
            json!({"name": "eDP-1", "enabled": true, "priority": 2}),
        ];

        let selected = select_target_output(&outputs, None).unwrap();
        assert_eq!(selected.get("name").unwrap(), "eDP-1");
    }

    #[test]
    fn parses_screen_index_from_support_information() {
        let text = "\
Screens
Number of Screens: 2

Screen 0:
----------
Name: eDP-1
Enabled: 1

Screen 1:
----------
Name: HDMI-A-1
Enabled: 1
";

        assert_eq!(parse_kwin_screen_index(text, "eDP-1"), Some(0));
        assert_eq!(parse_kwin_screen_index(text, "HDMI-A-1"), Some(1));
        assert_eq!(parse_kwin_screen_index(text, "DP-1"), None);
    }
}
