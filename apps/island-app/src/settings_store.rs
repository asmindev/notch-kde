use island_core::PersistedSettings;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SettingsStore;

impl SettingsStore {
    pub fn config_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|value| !value.is_empty())
                    .map(|home| PathBuf::from(home).join(".config"))
            })?;
        Some(base.join("notchde").join("settings.json"))
    }

    pub fn load() -> PersistedSettings {
        let Some(path) = Self::config_path() else {
            return PersistedSettings::default();
        };
        if !path.exists() {
            return PersistedSettings::default();
        }
        match Self::load_from(&path) {
            Ok(settings) => settings,
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "using default settings");
                PersistedSettings::default()
            }
        }
    }

    pub fn save(settings: &PersistedSettings) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Err(error) = Self::save_to(&path, settings) {
            tracing::warn!(path = %path.display(), %error, "failed to save settings");
        }
    }

    pub fn load_from(path: &Path) -> Result<PersistedSettings, String> {
        let data = fs::read(path).map_err(|error| error.to_string())?;
        serde_json::from_slice(&data).map_err(|error| error.to_string())
    }

    pub fn save_to(path: &Path, settings: &PersistedSettings) -> Result<(), String> {
        let parent = path
            .parent()
            .ok_or_else(|| "settings path has no parent".to_string())?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let data = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, data).map_err(|error| error.to_string())?;
        fs::rename(&temporary, path).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use island_core::{AvatarSettings, PetBehaviorSettings};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn temporary_path() -> PathBuf {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "notchde-settings-test-{}-{id}.json",
            std::process::id()
        ))
    }

    #[test]
    fn saves_and_loads_settings() {
        let path = temporary_path();
        let settings = PersistedSettings {
            avatar: AvatarSettings {
                character: 4,
                palette: 9,
                expression: 12,
                expression_custom: true,
                random: true,
            },
            behavior: PetBehaviorSettings {
                cursor_gaze: false,
                idle_acting: true,
                click_stunts: false,
                notification_reactions: true,
                music_reactions: false,
                sleepy_on_low_battery: true,
            },
            ..PersistedSettings::default()
        };

        SettingsStore::save_to(&path, &settings).expect("save settings");
        let loaded = SettingsStore::load_from(&path).expect("load settings");

        assert_eq!(loaded, settings);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn malformed_file_returns_error() {
        let path = temporary_path();
        fs::write(&path, b"not json").expect("write malformed settings");

        assert!(SettingsStore::load_from(&path).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_optional_fields_use_defaults() {
        let path = temporary_path();
        fs::write(&path, br#"{"version":1}"#).expect("write partial settings");

        let loaded = SettingsStore::load_from(&path).expect("load partial settings");
        assert_eq!(loaded, PersistedSettings::default());
        let _ = fs::remove_file(path);
    }
}
