use crate::models::MusicVisualMode;
use crate::prayer::PrayerSettings;
use crate::state::AppState;
use serde::{Deserialize, Serialize};

pub const SETTINGS_VERSION: u32 = 1;
pub const SHAPE_COUNT: u32 = 8;
pub const PALETTE_COUNT: u32 = 14;
pub const EXPRESSION_COUNT: u32 = 17;
pub const SHADOW_INTENSITY_MAX: u32 = 100;
pub const SHADOW_RADIUS_MAX: u32 = 48;
pub const SHADOW_OFFSET_MAX: i32 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(i32)]
pub enum SettingsTab {
    #[default]
    Avatar = 0,
    Behavior = 1,
    Prayer = 2,
    System = 3,
    About = 4,
}

impl SettingsTab {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Behavior,
            2 => Self::Prayer,
            3 => Self::System,
            4 => Self::About,
            _ => Self::Avatar,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PetRenderer {
    #[default]
    #[serde(rename = "3d")]
    ThreeD,
    #[serde(rename = "2d")]
    TwoD,
}

impl PetRenderer {
    pub fn is_3d(self) -> bool {
        matches!(self, Self::ThreeD)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CompactModeStyle {
    #[default]
    #[serde(rename = "standard")]
    Standard = 0,
    #[serde(rename = "eyes_only")]
    EyesOnly = 1,
}

impl CompactModeStyle {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Self::EyesOnly,
            _ => Self::Standard,
        }
    }

    pub fn is_eyes_only(self) -> bool {
        matches!(self, Self::EyesOnly)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AvatarSettings {
    pub character: u32,
    pub palette: u32,
    pub expression: u32,
    pub expression_custom: bool,
    pub random: bool,
}

impl Default for AvatarSettings {
    fn default() -> Self {
        Self {
            character: 1,
            palette: 0,
            expression: 0,
            expression_custom: false,
            random: false,
        }
    }
}

impl AvatarSettings {
    pub fn normalized(mut self) -> Self {
        self.character %= SHAPE_COUNT;
        self.palette %= PALETTE_COUNT;
        self.expression %= EXPRESSION_COUNT;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PetBehaviorSettings {
    pub cursor_gaze: bool,
    pub idle_acting: bool,
    pub click_stunts: bool,
    pub notification_reactions: bool,
    pub music_reactions: bool,
    pub sleepy_on_low_battery: bool,
}

impl Default for PetBehaviorSettings {
    fn default() -> Self {
        Self {
            cursor_gaze: true,
            idle_acting: true,
            click_stunts: true,
            notification_reactions: true,
            music_reactions: true,
            sleepy_on_low_battery: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShadowSettings {
    pub enabled: bool,
    pub intensity: u32,
    pub radius: u32,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl Default for ShadowSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity: 55,
            radius: 18,
            offset_x: 0,
            offset_y: 6,
        }
    }
}

impl ShadowSettings {
    pub fn normalized(mut self) -> Self {
        self.intensity = self.intensity.min(SHADOW_INTENSITY_MAX);
        self.radius = self.radius.min(SHADOW_RADIUS_MAX);
        self.offset_x = self.offset_x.clamp(-SHADOW_OFFSET_MAX, SHADOW_OFFSET_MAX);
        self.offset_y = self.offset_y.clamp(-SHADOW_OFFSET_MAX, SHADOW_OFFSET_MAX);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistedSettings {
    pub version: u32,
    pub avatar: AvatarSettings,
    pub behavior: PetBehaviorSettings,
    pub shadow: ShadowSettings,
    pub media_visual_mode: MusicVisualMode,
    pub pet_renderer: Option<PetRenderer>,
    pub prayer: PrayerSettings,
    pub compact_mode_style: CompactModeStyle,
}

impl Default for PersistedSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            avatar: AvatarSettings::default(),
            behavior: PetBehaviorSettings::default(),
            shadow: ShadowSettings::default(),
            media_visual_mode: MusicVisualMode::ArtworkPrimary,
            pet_renderer: None,
            prayer: PrayerSettings::default(),
            compact_mode_style: CompactModeStyle::default(),
        }
    }
}

/// Migrates persisted settings from an untrusted JSON value.
///
/// Returns the migrated settings and whether the stored version changed.
pub fn migrate_settings_value(
    mut value: serde_json::Value,
) -> Result<(PersistedSettings, bool), String> {
    let stored_version = match value.get("version") {
        Some(version) => version
            .as_u64()
            .ok_or_else(|| "settings version must be an unsigned integer".to_string())?
            as u32,
        None => 0,
    };

    if stored_version > SETTINGS_VERSION {
        return Err(format!(
            "unsupported settings version {stored_version} (current {SETTINGS_VERSION})"
        ));
    }

    let mut version = stored_version;
    while version < SETTINGS_VERSION {
        match version {
            0 => {
                let object = value
                    .as_object_mut()
                    .ok_or_else(|| "settings root must be an object".to_string())?;
                object.insert("version".to_string(), serde_json::json!(1));
                version = 1;
            }
            other => {
                return Err(format!(
                    "no migration path from settings version {other} to {SETTINGS_VERSION}"
                ));
            }
        }
    }

    let mut settings: PersistedSettings =
        serde_json::from_value(value).map_err(|error| format!("invalid settings: {error}"))?;
    settings.version = SETTINGS_VERSION;
    Ok((settings, stored_version < SETTINGS_VERSION))
}

impl PersistedSettings {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            version: SETTINGS_VERSION,
            avatar: AvatarSettings {
                character: state.settings.blob_character,
                palette: state.settings.blob_palette,
                expression: state.settings.blob_expression,
                expression_custom: state.settings.blob_expression_custom,
                random: state.settings.avatar_random,
            }
            .normalized(),
            behavior: state.settings.behavior,
            shadow: state.settings.shadow.normalized(),
            media_visual_mode: state.settings.music_visual_mode,
            pet_renderer: Some(state.settings.pet_renderer),
            prayer: state.settings.prayer.normalized(),
            compact_mode_style: state.settings.compact_mode_style,
        }
    }

    pub fn apply_to_state(&self, state: &mut AppState) {
        let avatar = self.avatar.normalized();
        state.settings.blob_character = avatar.character;
        state.settings.blob_palette = avatar.palette;
        state.settings.blob_expression = avatar.expression;
        state.settings.blob_expression_custom = avatar.expression_custom;
        state.settings.avatar_random = avatar.random;
        state.settings.behavior = self.behavior;
        state.settings.shadow = self.shadow.normalized();
        state.settings.music_visual_mode = self.media_visual_mode;
        if let Some(renderer) = self.pet_renderer {
            state.settings.pet_renderer = renderer;
        }
        state.settings.prayer = self.prayer.normalized();
        state.settings.compact_mode_style = self.compact_mode_style;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_stable_and_normalized() {
        let settings = PersistedSettings::default();
        assert_eq!(settings.version, SETTINGS_VERSION);
        assert_eq!(settings.avatar.character, 1);
        assert_eq!(settings.avatar.palette, 0);
        assert_eq!(settings.avatar.expression, 0);
        assert!(settings.behavior.cursor_gaze);
        assert!(settings.behavior.idle_acting);
        assert!(settings.shadow.enabled);
        assert_eq!(settings.shadow.intensity, 55);
        assert_eq!(settings.shadow.radius, 18);
        assert_eq!(settings.shadow.offset_x, 0);
        assert_eq!(settings.shadow.offset_y, 6);
        assert_eq!(settings.pet_renderer, None);
        assert!(settings.prayer.enabled);
        assert_eq!(settings.prayer.city, 0);
        assert_eq!(settings.prayer.reminder_minutes, 10);
    }

    #[test]
    fn apply_clamps_out_of_range_avatar_values() {
        let mut state = AppState::default();
        let settings = PersistedSettings {
            avatar: AvatarSettings {
                character: 99,
                palette: 99,
                expression: 99,
                expression_custom: true,
                random: true,
            },
            ..PersistedSettings::default()
        };

        settings.apply_to_state(&mut state);

        assert_eq!(state.settings.blob_character, 99 % SHAPE_COUNT);
        assert_eq!(state.settings.blob_palette, 99 % PALETTE_COUNT);
        assert_eq!(state.settings.blob_expression, 99 % EXPRESSION_COUNT);
        assert!(state.settings.blob_expression_custom);
        assert!(state.settings.avatar_random);
    }

    #[test]
    fn missing_persisted_renderer_preserves_config_default() {
        let mut state = AppState::default();
        state.settings.pet_renderer = PetRenderer::TwoD;

        PersistedSettings::default().apply_to_state(&mut state);

        assert_eq!(state.settings.pet_renderer, PetRenderer::TwoD);
    }

    #[test]
    fn round_trips_state_settings() {
        let mut state = AppState::default();
        state.settings.blob_character = 5;
        state.settings.blob_palette = 7;
        state.settings.blob_expression = 11;
        state.settings.blob_expression_custom = true;
        state.settings.avatar_random = true;
        state.settings.behavior.cursor_gaze = false;
        state.settings.behavior.music_reactions = false;
        state.settings.shadow.enabled = false;
        state.settings.shadow.intensity = 80;
        state.settings.shadow.radius = 34;
        state.settings.shadow.offset_x = -8;
        state.settings.shadow.offset_y = 12;
        state.settings.music_visual_mode = MusicVisualMode::PetPrimary;
        state.settings.pet_renderer = PetRenderer::TwoD;
        state.settings.prayer.enabled = false;
        state.settings.prayer.city = 8;
        state.settings.prayer.reminder_minutes = 15;
        state.settings.compact_mode_style = CompactModeStyle::EyesOnly;

        let snapshot = PersistedSettings::from_state(&state);
        let mut restored = AppState::default();
        snapshot.apply_to_state(&mut restored);

        assert_eq!(restored.settings.blob_character, 5);
        assert_eq!(restored.settings.blob_palette, 7);
        assert_eq!(restored.settings.blob_expression, 11);
        assert!(restored.settings.blob_expression_custom);
        assert!(restored.settings.avatar_random);
        assert!(!restored.settings.behavior.cursor_gaze);
        assert!(!restored.settings.behavior.music_reactions);
        assert!(!restored.settings.shadow.enabled);
        assert_eq!(restored.settings.shadow.intensity, 80);
        assert_eq!(restored.settings.shadow.radius, 34);
        assert_eq!(restored.settings.shadow.offset_x, -8);
        assert_eq!(restored.settings.shadow.offset_y, 12);
        assert_eq!(
            restored.settings.music_visual_mode,
            MusicVisualMode::PetPrimary
        );
        assert_eq!(restored.settings.pet_renderer, PetRenderer::TwoD);
        assert!(!restored.settings.prayer.enabled);
        assert_eq!(restored.settings.prayer.city, 8);
        assert_eq!(restored.settings.prayer.reminder_minutes, 15);
        assert_eq!(
            restored.settings.compact_mode_style,
            CompactModeStyle::EyesOnly
        );
    }

    #[test]
    fn compact_mode_style_index_mapping_and_serialization() {
        assert_eq!(CompactModeStyle::from_index(0), CompactModeStyle::Standard);
        assert_eq!(CompactModeStyle::from_index(1), CompactModeStyle::EyesOnly);
        assert_eq!(CompactModeStyle::from_index(99), CompactModeStyle::Standard);

        assert!(!CompactModeStyle::Standard.is_eyes_only());
        assert!(CompactModeStyle::EyesOnly.is_eyes_only());

        let json = serde_json::to_string(&CompactModeStyle::EyesOnly).unwrap();
        assert_eq!(json, "\"eyes_only\"");
        let deserialized: CompactModeStyle = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, CompactModeStyle::EyesOnly);
    }

    #[test]
    fn migrates_unversioned_settings_to_current_version() {
        let (settings, migrated) =
            migrate_settings_value(serde_json::json!({"shadow": {"intensity": 80}}))
                .expect("migrate settings");

        assert!(migrated);
        assert_eq!(settings.version, SETTINGS_VERSION);
        assert_eq!(settings.shadow.intensity, 80);
    }

    #[test]
    fn rejects_unknown_future_settings_version() {
        let result = migrate_settings_value(serde_json::json!({
            "version": SETTINGS_VERSION + 1
        }));

        assert!(result.is_err());
    }
}
