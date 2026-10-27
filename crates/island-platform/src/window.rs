use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tracing::{info, warn};

pub struct PlatformWindow;

impl PlatformWindow {
    /// Configures KWin window rules to anchor the island at the target screen,
    /// keep it frameless, always on top, and excluded from taskbars and pagers.
    pub fn ensure_kwin_rules(window_x: i32, window_y: i32, screen_index: u32) -> Result<(), String> {
        let pos = format!("{},{}", window_x, window_y);
        let target_screen_idx = screen_index.to_string();

        let home = std::env::var("HOME").map_err(|e| e.to_string())?;
        let config_path = PathBuf::from(home).join(".config/kwinrulesrc");

        let existing_content = fs::read_to_string(&config_path).unwrap_or_default();

        let target_group = "[island-app-rule]";
        let target_screen = format!("screen={}", target_screen_idx);
        let target_pos = format!("position={}", pos);

        if existing_content.contains(target_group)
            && existing_content.contains(&target_screen)
            && existing_content.contains(&target_pos)
            && existing_content.contains("screenrule=2")
        {
            info!("KWin window rule for island-app already up-to-date at primary screen (screen: {}, pos: {}). Skipping reconfigure.", target_screen_idx, pos);
            return Ok(());
        }

        info!("Writing KWin window rules for primary screen (screen: {}) at pos: {}", target_screen_idx, pos);

        let rule_body = format!(
            "[island-app-rule]\n\
Description=KDE Dynamic Island Placement\n\
above=true\n\
aboverule=2\n\
decorationpolicy=none\n\
decorationpolicyrule=2\n\
position={pos}\n\
positionrule=2\n\
screen={target_screen_idx}\n\
screenrule=2\n\
skippager=true\n\
skippagerrule=2\n\
skipswitcher=true\n\
skipswitcherrule=2\n\
skiptaskbar=true\n\
skiptaskbarrule=2\n\
wmclass=island-app\n\
wmclassmatch=1\n"
        );

        let mut new_lines = Vec::new();
        let mut in_group = false;
        let mut group_found = false;

        for line in existing_content.lines() {
            if line.trim().starts_with('[') {
                if in_group {
                    in_group = false;
                }
                if line.trim() == target_group {
                    in_group = true;
                    group_found = true;
                    new_lines.push(rule_body.trim().to_string());
                    continue;
                }
            }

            if in_group {
                continue;
            }

            if line.starts_with("order=") || line.starts_with("Order=") {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    let key = parts[0];
                    let val = parts[1];
                    if !val.contains("island-app-rule") {
                        if val.is_empty() {
                            new_lines.push(format!("{}=island-app-rule", key));
                        } else {
                            new_lines.push(format!("{},island-app-rule", line));
                        }
                        continue;
                    }
                }
            }

            new_lines.push(line.to_string());
        }

        if !group_found {
            if !new_lines.is_empty() && !new_lines.last().unwrap().is_empty() {
                new_lines.push(String::new());
            }
            new_lines.push(rule_body.trim().to_string());
        }

        let final_content = new_lines.join("\n") + "\n";
        if let Err(e) = fs::write(&config_path, final_content) {
            warn!("Failed to write kwinrulesrc directly: {}", e);
        } else {
            info!("Successfully updated kwinrulesrc directly with primary screen rules");
        }

        // Notify KWin to reload rules immediately
        let _ = Command::new("qdbus6")
            .args(["org.kde.KWin", "/KWin", "reconfigure"])
            .status()
            .or_else(|_| {
                Command::new("qdbus")
                    .args(["org.kde.KWin", "/KWin", "reconfigure"])
                    .status()
            });

        Ok(())
    }
}
