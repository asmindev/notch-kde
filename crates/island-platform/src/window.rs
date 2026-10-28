use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info, warn};

const RULE_GROUP: &str = "island-app-rule";

pub struct PlatformWindow;

impl PlatformWindow {
    pub fn ensure_kwin_rules(
        window_x: i32,
        window_y: i32,
        screen_index: u32,
    ) -> Result<bool, String> {
        let pos = format!("{},{}", window_x, window_y);
        let home = std::env::var("HOME").map_err(|e| e.to_string())?;
        let config_path = PathBuf::from(home).join(".config/kwinrulesrc");
        let existing_content = fs::read_to_string(&config_path).unwrap_or_default();

        let mut rules = collect_rule_groups(&existing_content);
        if !rules.iter().any(|rule| rule == RULE_GROUP) {
            rules.push(RULE_GROUP.to_string());
        }

        if is_rule_current(&existing_content, RULE_GROUP, &rules, &pos, screen_index) {
            debug!(
                "KWin window rule for island-app already up-to-date (screen: {}, pos: {})",
                screen_index, pos
            );
            return Ok(false);
        }

        let rule_body = format!(
            "Description=KDE Dynamic Island Placement\n\
above=true\n\
aboverule=2\n\
acceptfocus=false\n\
acceptfocusrule=2\n\
decorationpolicy=none\n\
decorationpolicyrule=2\n\
layer=notification\n\
layerrule=2\n\
position={pos}\n\
positionrule=2\n\
screen={screen_index}\n\
screenrule=2\n\
desktops=\n\
desktopsrule=2\n\
skippager=true\n\
skippagerrule=2\n\
skipswitcher=true\n\
skipswitcherrule=2\n\
skiptaskbar=true\n\
skiptaskbarrule=2\n\
title=island-app\n\
titlematch=1\n\
wmclass=island-app\n\
wmclassmatch=1\n"
        );

        let final_content = render_config(&existing_content, RULE_GROUP, &rule_body, &rules);

        if let Err(e) = fs::write(&config_path, final_content) {
            warn!("Failed to write kwinrulesrc directly: {}", e);
        } else {
            info!(
                "Updated kwinrulesrc with KWin 6.8 rule list and all-desktop placement (screen: {}, pos: {})",
                screen_index, pos
            );
        }

        notify_kwin_reconfigure();
        Ok(true)
    }
}

fn notify_kwin_reconfigure() {
    let _ = Command::new("qdbus6")
        .args(["org.kde.KWin", "/KWin", "reconfigure"])
        .status()
        .or_else(|_| {
            Command::new("qdbus")
                .args(["org.kde.KWin", "/KWin", "reconfigure"])
                .status()
        });
}

fn collect_rule_groups(content: &str) -> Vec<String> {
    let mut rules = None;
    let mut legacy_upper = None;
    let mut legacy_lower = None;
    let mut in_general = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            in_general = line == "[General]";
            continue;
        }

        if !in_general {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let groups: Vec<String> = value
            .split(',')
            .map(str::trim)
            .filter(|group| !group.is_empty())
            .map(str::to_string)
            .collect();

        if groups.is_empty() {
            continue;
        }

        match key.trim() {
            "rules" => rules = Some(groups),
            "Order" => legacy_upper = Some(groups),
            "order" => legacy_lower = Some(groups),
            _ => {}
        }
    }

    let mut result = Vec::new();
    for group in rules
        .into_iter()
        .flatten()
        .chain(legacy_upper.into_iter().flatten())
        .chain(legacy_lower.into_iter().flatten())
    {
        if !result.contains(&group) {
            result.push(group);
        }
    }
    result
}

fn render_config(existing: &str, target_group: &str, rule_body: &str, rules: &[String]) -> String {
    let target_section = format!("[{}]", target_group);
    let mut output = Vec::new();
    let mut in_general = false;
    let mut in_target = false;
    let mut general_seen = false;
    let mut target_seen = false;
    let mut rules_written = false;

    for line in existing.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_general = trimmed == "[General]";
            in_target = trimmed == target_section;

            if in_target {
                target_seen = true;
                output.push(target_section.clone());
                output.extend(rule_body.lines().map(str::to_string));
                continue;
            }

            output.push(line.to_string());

            if in_general {
                general_seen = true;
                if !rules_written {
                    output.push(format!("rules={}", rules.join(",")));
                    rules_written = true;
                }
            }
            continue;
        }

        if in_target {
            continue;
        }

        if in_general {
            let key = trimmed
                .split_once('=')
                .map(|(key, _)| key.trim())
                .unwrap_or("");
            if key == "rules" || key == "Order" || key == "order" {
                continue;
            }
        }

        output.push(line.to_string());
    }

    if !general_seen {
        let mut with_general = vec![
            "[General]".to_string(),
            format!("rules={}", rules.join(",")),
        ];
        if !output.is_empty() {
            with_general.push(String::new());
        }
        with_general.extend(output);
        output = with_general;
    }

    if !target_seen {
        if !output.is_empty() && !output.last().is_some_and(String::is_empty) {
            output.push(String::new());
        }
        output.push(target_section);
        output.extend(rule_body.lines().map(str::to_string));
    }

    output.join("\n") + "\n"
}

fn is_rule_current(
    content: &str,
    target_group: &str,
    rules: &[String],
    pos: &str,
    screen_index: u32,
) -> bool {
    if general_rule_groups(content) != rules {
        return false;
    }

    let Some(body) = section_body(content, target_group) else {
        return false;
    };
    let entries = section_entries(&body);

    let expected = [
        ("above", "true".to_string()),
        ("aboverule", "2".to_string()),
        ("acceptfocus", "false".to_string()),
        ("acceptfocusrule", "2".to_string()),
        ("decorationpolicy", "none".to_string()),
        ("decorationpolicyrule", "2".to_string()),
        ("layer", "notification".to_string()),
        ("layerrule", "2".to_string()),
        ("position", pos.to_string()),
        ("positionrule", "2".to_string()),
        ("screen", screen_index.to_string()),
        ("screenrule", "2".to_string()),
        ("desktops", String::new()),
        ("desktopsrule", "2".to_string()),
        ("skippager", "true".to_string()),
        ("skippagerrule", "2".to_string()),
        ("skipswitcher", "true".to_string()),
        ("skipswitcherrule", "2".to_string()),
        ("skiptaskbar", "true".to_string()),
        ("skiptaskbarrule", "2".to_string()),
        ("title", "island-app".to_string()),
        ("titlematch", "1".to_string()),
        ("wmclass", "island-app".to_string()),
        ("wmclassmatch", "1".to_string()),
    ];

    expected
        .iter()
        .all(|(key, value)| entries.get(*key) == Some(value))
}

fn general_rule_groups(content: &str) -> Vec<String> {
    let mut in_general = false;
    let mut rules = None;
    let mut order = None;
    let mut lower_order = None;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            in_general = line == "[General]";
            continue;
        }

        if !in_general {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let groups = || {
            value
                .split(',')
                .map(str::trim)
                .filter(|group| !group.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        };

        match key.trim() {
            "rules" => rules = Some(groups()),
            "Order" => order = Some(groups()),
            "order" => lower_order = Some(groups()),
            _ => {}
        }
    }

    rules.or(order).or(lower_order).unwrap_or_default()
}

fn section_body(content: &str, target_group: &str) -> Option<String> {
    let target_section = format!("[{}]", target_group);
    let mut in_target = false;
    let mut body = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_target {
                break;
            }
            in_target = trimmed == target_section;
            continue;
        }

        if in_target {
            body.push(line.to_string());
        }
    }

    in_target.then(|| body.join("\n"))
}

fn section_entries(body: &str) -> HashMap<String, String> {
    body.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY: &str = "\
[General]
Order=island-app-rule,steam,notch-spike-rule
order=steam,island-app-rule

[island-app-rule]
position=1,2
screen=0

[steam]
enabled=false
";

    #[test]
    fn collects_legacy_rule_order_without_duplicates() {
        assert_eq!(
            collect_rule_groups(LEGACY),
            vec!["island-app-rule", "steam", "notch-spike-rule"]
        );
    }

    #[test]
    fn renders_new_rule_list_and_preserves_other_groups() {
        let rendered = render_config(
            LEGACY,
            RULE_GROUP,
            "position=10,20\nscreen=1\ndesktops=\ndesktopsrule=2\n",
            &collect_rule_groups(LEGACY),
        );

        assert!(rendered.contains("rules=island-app-rule,steam,notch-spike-rule"));
        assert!(!rendered.contains("Order="));
        assert!(!rendered.contains("\norder="));
        assert!(rendered.contains("[steam]"));
        assert!(rendered.contains("enabled=false"));
        assert!(rendered.contains("position=10,20"));
        assert!(rendered.contains("desktops=\ndesktopsrule=2"));
    }

    #[test]
    fn creates_general_section_when_missing() {
        let rendered = render_config("", RULE_GROUP, "position=0,0\n", &[]);

        assert!(rendered.starts_with("[General]\nrules=\n"));
        assert!(rendered.contains("[island-app-rule]"));
        assert!(rendered.contains("position=0,0"));
    }

    #[test]
    fn kwin_order_key_is_accepted_as_current() {
        let rendered = render_config(
            LEGACY,
            RULE_GROUP,
            "Description=KDE Dynamic Island Placement\n\
above=true\n\
aboverule=2\n\
acceptfocus=false\n\
acceptfocusrule=2\n\
decorationpolicy=none\n\
decorationpolicyrule=2\n\
layer=notification\n\
layerrule=2\n\
position=10,20\n\
positionrule=2\n\
screen=1\n\
screenrule=2\n\
desktops=\n\
desktopsrule=2\n\
skippager=true\n\
skippagerrule=2\n\
skipswitcher=true\n\
skipswitcherrule=2\n\
skiptaskbar=true\n\
skiptaskbarrule=2\n\
title=island-app\n\
titlematch=1\n\
wmclass=island-app\n\
wmclassmatch=1\n",
            &collect_rule_groups(LEGACY),
        );
        let order = rendered.replace(
            "rules=island-app-rule,steam,notch-spike-rule",
            "Order=island-app-rule,steam,notch-spike-rule",
        );

        assert!(is_rule_current(
            &order,
            RULE_GROUP,
            &collect_rule_groups(&order),
            "10,20",
            1
        ));
    }

    #[test]
    fn detects_current_rule() {
        let rendered = render_config(
            LEGACY,
            RULE_GROUP,
            "Description=KDE Dynamic Island Placement\n\
above=true\n\
aboverule=2\n\
acceptfocus=false\n\
acceptfocusrule=2\n\
decorationpolicy=none\n\
decorationpolicyrule=2\n\
layer=notification\n\
layerrule=2\n\
position=10,20\n\
positionrule=2\n\
screen=1\n\
screenrule=2\n\
desktops=\n\
desktopsrule=2\n\
skippager=true\n\
skippagerrule=2\n\
skipswitcher=true\n\
skipswitcherrule=2\n\
skiptaskbar=true\n\
skiptaskbarrule=2\n\
title=island-app\n\
titlematch=1\n\
wmclass=island-app\n\
wmclassmatch=1\n",
            &collect_rule_groups(LEGACY),
        );

        assert!(is_rule_current(
            &rendered,
            RULE_GROUP,
            &collect_rule_groups(&rendered),
            "10,20",
            1
        ));
    }
}
