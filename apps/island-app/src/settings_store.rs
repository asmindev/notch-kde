use island_core::{PersistedSettings, migrate_settings_value};
use std::fs;
use std::path::Path;

pub struct SettingsStore;

impl SettingsStore {
    pub fn load(path: &Path) -> PersistedSettings {
        if !path.exists() {
            return PersistedSettings::default();
        }
        match Self::load_versioned_from(path) {
            Ok((settings, migrated)) => {
                if migrated && let Err(error) = Self::save_to(path, &settings) {
                    tracing::warn!(
                        path = %path.display(),
                        %error,
                        "failed to persist migrated settings"
                    );
                }
                settings
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "using default settings");
                PersistedSettings::default()
            }
        }
    }

    pub fn save(path: &Path, settings: &PersistedSettings) {
        if let Err(error) = Self::save_to(path, settings) {
            tracing::warn!(path = %path.display(), %error, "failed to save settings");
        }
    }

    #[cfg(test)]
    pub fn load_from(path: &Path) -> Result<PersistedSettings, String> {
        Self::load_versioned_from(path).map(|(settings, _)| settings)
    }

    pub fn load_versioned_from(path: &Path) -> Result<(PersistedSettings, bool), String> {
        let data = fs::read(path).map_err(|error| error.to_string())?;
        let value = serde_json::from_slice(&data).map_err(|error| error.to_string())?;
        migrate_settings_value(value)
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
    use island_core::settings::SETTINGS_VERSION;
    use island_core::{AvatarSettings, PetBehaviorSettings, ShadowSettings};
    use std::path::PathBuf;
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
            shadow: ShadowSettings {
                enabled: false,
                intensity: 80,
                radius: 34,
                offset_x: -8,
                offset_y: 12,
            },
            ..PersistedSettings::default()
        };

        SettingsStore::save_to(&path, &settings).expect("save settings");
        let loaded = SettingsStore::load_from(&path).expect("load settings");

        assert_eq!(loaded, settings);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn loads_persisted_pet_renderer() {
        let path = temporary_path();
        fs::write(&path, br#"{"version":1,"pet_renderer":"2d"}"#)
            .expect("write settings with pet renderer");

        let loaded = SettingsStore::load_from(&path).expect("load pet renderer");

        assert_eq!(loaded.pet_renderer, Some(island_core::PetRenderer::TwoD));
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

    #[test]
    fn load_persists_migrated_version() {
        let path = temporary_path();
        fs::write(&path, br#"{"shadow":{"intensity":80}}"#).expect("write legacy settings");

        let (settings, migrated) =
            SettingsStore::load_versioned_from(&path).expect("migrate settings");
        assert!(migrated);
        assert_eq!(settings.version, SETTINGS_VERSION);

        SettingsStore::load(&path);
        let data: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read migrated settings"))
                .expect("parse migrated settings");
        assert_eq!(data["version"], SETTINGS_VERSION);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn unknown_future_version_falls_back() {
        let path = temporary_path();
        fs::write(&path, br#"{"version":999}"#).expect("write future settings");

        let loaded = SettingsStore::load(&path);

        assert_eq!(loaded, PersistedSettings::default());
        let _ = fs::remove_file(path);
    }
}
