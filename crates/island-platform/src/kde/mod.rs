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

impl ScreenGeometry {
    pub fn fallback() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            screen_index: 0,
        }
    }
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

    /// Queries the settled priority-1 screen geometry (global x, y, logical width, height, screen index).
    ///
    /// Returns `None` when KDE tools are unavailable or no primary output can be resolved.
    pub fn get_primary_screen_geometry() -> Option<ScreenGeometry> {
        let geometry = Self::read_stable_kscreen_doctor()?;
        info!(
            "Detected primary screen geometry via kscreen-doctor: pos=({},{}) size={}x{} (screen: {})",
            geometry.x, geometry.y, geometry.width, geometry.height, geometry.screen_index
        );
        Some(geometry)
    }

    /// Non-panicking variant for runtime screen-layout watches.
    pub fn try_get_primary_screen_geometry() -> Option<ScreenGeometry> {
        Self::read_stable_kscreen_doctor()
    }

    /// Backwards compatibility helper returning (logical width, logical height).
    pub fn get_primary_screen_size() -> Option<(u32, u32)> {
        Self::get_primary_screen_geometry().map(|geometry| (geometry.width, geometry.height))
    }

    fn read_stable_kscreen_doctor() -> Option<ScreenGeometry> {
        let mut last = None;
        let mut stable_reads = 0;

        for _ in 0..30 {
            let current = Self::read_kscreen_doctor();

            if current.is_some() && current == last {
                stable_reads += 1;
                if stable_reads >= 2 {
                    return current;
                }
            } else {
                stable_reads = 0;
            }

            last = current;
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        last
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
        let target_output = select_primary_output(outputs)?;
        let name = target_output.get("name")?.as_str()?;

        let enabled_outputs = outputs
            .iter()
            .filter(|output| {
                output
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .count();

        let kwin_screens = read_kwin_screens()?;
        let active_screens: Vec<_> = kwin_screens
            .iter()
            .filter(|screen| screen.enabled)
            .collect();
        if active_screens.len() != enabled_outputs {
            return None;
        }

        let screen = active_screens.iter().find(|screen| screen.name == name)?;

        Some(ScreenGeometry {
            x: screen.x,
            y: screen.y,
            width: screen.width,
            height: screen.height,
            screen_index: screen.index,
        })
    }
}

fn select_primary_output<'a>(outputs: &'a [Value]) -> Option<&'a Value> {
    outputs
        .iter()
        .find(|output| output.get("priority").and_then(Value::as_u64) == Some(1))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KWinScreen {
    index: u32,
    name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    enabled: bool,
}

fn read_kwin_screens() -> Option<Vec<KWinScreen>> {
    for command in ["qdbus6", "qdbus"] {
        let output = match std::process::Command::new(command)
            .args(["org.kde.KWin", "/KWin", "supportInformation"])
            .output()
        {
            Ok(output) if output.status.success() => output,
            _ => continue,
        };

        let text = String::from_utf8_lossy(&output.stdout);
        let screens = parse_kwin_screens(&text);
        if !screens.is_empty() {
            return Some(screens);
        }
    }
    None
}

fn parse_kwin_screens(text: &str) -> Vec<KWinScreen> {
    let mut screens = Vec::new();
    let mut current: Option<(u32, Option<String>, Option<(i32, i32, u32, u32)>, bool)> = None;

    for line in text.lines() {
        let line = line.trim();

        if let Some(number) = line
            .strip_prefix("Screen ")
            .and_then(|rest| rest.strip_suffix(':'))
        {
            if let Some((index, Some(name), Some((x, y, width, height)), enabled)) = current.take()
            {
                screens.push(KWinScreen {
                    index,
                    name,
                    x,
                    y,
                    width,
                    height,
                    enabled,
                });
            }

            current = number
                .trim()
                .parse::<u32>()
                .ok()
                .map(|index| (index, None, None, true));
            continue;
        }

        let Some((_, name, geometry, enabled)) = current.as_mut() else {
            continue;
        };

        if let Some(rest) = line.strip_prefix("Name:") {
            *name = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("Geometry:") {
            *geometry = parse_kwin_geometry(rest.trim());
        } else if let Some(rest) = line.strip_prefix("Enabled:") {
            *enabled = matches!(rest.trim(), "1" | "true");
        }
    }

    if let Some((index, Some(name), Some((x, y, width, height)), enabled)) = current.take() {
        screens.push(KWinScreen {
            index,
            name,
            x,
            y,
            width,
            height,
            enabled,
        });
    }

    screens
}

fn parse_kwin_geometry(text: &str) -> Option<(i32, i32, u32, u32)> {
    let mut parts = text.split(',');
    let x = parts.next()?.trim().parse::<i32>().ok()?;
    let y = parts.next()?.trim().parse::<i32>().ok()?;
    let (width, height) = parts.next()?.trim().split_once('x')?;

    Some((
        x,
        y,
        width.trim().parse::<u32>().ok()?,
        height.trim().parse::<u32>().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn selects_priority_one() {
        let outputs = vec![
            json!({"name": "eDP-1", "enabled": true, "priority": 2}),
            json!({"name": "HDMI-A-1", "enabled": true, "priority": 1}),
        ];

        let selected = select_primary_output(&outputs).unwrap();
        assert_eq!(selected.get("name").unwrap(), "HDMI-A-1");
    }

    #[test]
    fn priority_one_is_used_even_if_disabled() {
        let outputs = vec![
            json!({"name": "HDMI-A-1", "enabled": false, "priority": 1}),
            json!({"name": "eDP-1", "enabled": true, "priority": 2}),
        ];

        let selected = select_primary_output(&outputs).unwrap();
        assert_eq!(selected.get("name").unwrap(), "HDMI-A-1");
    }

    #[test]
    fn no_priority_one_returns_none() {
        let outputs = vec![
            json!({"name": "eDP-1", "enabled": true, "priority": 2}),
            json!({"name": "HDMI-A-1", "enabled": true, "priority": 3}),
        ];

        assert!(select_primary_output(&outputs).is_none());
    }

    #[test]
    fn parses_kwin_screens_with_geometry() {
        let text = "\
Screens
=======
Number of Screens: 2

Screen 0:
---------
Name: eDP-1
Enabled: 1
Geometry: 1746,34,1516x947
Scale: 1.9

Screen 1:
---------
Name: HDMI-A-1
Enabled: 1
Geometry: 0,0,1745x982
Scale: 1.1
";

        let screens = parse_kwin_screens(text);
        assert_eq!(screens.len(), 2);
        assert_eq!(screens[0].index, 0);
        assert_eq!(screens[0].name, "eDP-1");
        assert_eq!(
            (
                screens[0].x,
                screens[0].y,
                screens[0].width,
                screens[0].height
            ),
            (1746, 34, 1516, 947)
        );
        assert_eq!(screens[1].index, 1);
        assert_eq!(screens[1].name, "HDMI-A-1");
        assert_eq!(
            (
                screens[1].x,
                screens[1].y,
                screens[1].width,
                screens[1].height
            ),
            (0, 0, 1745, 982)
        );
    }
}
