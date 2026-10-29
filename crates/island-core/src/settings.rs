use crate::models::MusicVisualMode;
use crate::prayer::PrayerSettings;
use crate::state::AppState;
use serde::{Deserialize, Serialize};

pub const SETTINGS_VERSION: u32 = 1;
pub const SHAPE_COUNT: u32 = 8;
pub const PALETTE_COUNT: u32 = 14;
pub const EXPRESSION_COUNT: u32 = 17;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistedSettings {
    pub version: u32,
    pub avatar: AvatarSettings,
    pub behavior: PetBehaviorSettings,
    pub media_visual_mode: MusicVisualMode,
    pub prayer: PrayerSettings,
}

impl Default for PersistedSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            avatar: AvatarSettings::default(),
            behavior: PetBehaviorSettings::default(),
            media_visual_mode: MusicVisualMode::ArtworkPrimary,
            prayer: PrayerSettings::default(),
        }
    }
}

impl PersistedSettings {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            version: SETTINGS_VERSION,
            avatar: AvatarSettings {
                character: state.blob_character,
                palette: state.blob_palette,
                expression: state.blob_expression,
                expression_custom: state.blob_expression_custom,
                random: state.avatar_random,
            }
            .normalized(),
            behavior: state.behavior,
            media_visual_mode: state.music_visual_mode,
            prayer: state.prayer.normalized(),
        }
    }

    pub fn apply_to_state(&self, state: &mut AppState) {
        let avatar = self.avatar.normalized();
        state.blob_character = avatar.character;
        state.blob_palette = avatar.palette;
        state.blob_expression = avatar.expression;
        state.blob_expression_custom = avatar.expression_custom;
        state.avatar_random = avatar.random;
        state.behavior = self.behavior;
        state.music_visual_mode = self.media_visual_mode;
        state.prayer = self.prayer.normalized();
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

        assert_eq!(state.blob_character, 99 % SHAPE_COUNT);
        assert_eq!(state.blob_palette, 99 % PALETTE_COUNT);
        assert_eq!(state.blob_expression, 99 % EXPRESSION_COUNT);
        assert!(state.blob_expression_custom);
        assert!(state.avatar_random);
    }

    #[test]
    fn round_trips_state_settings() {
        let mut state = AppState::default();
        state.blob_character = 5;
        state.blob_palette = 7;
        state.blob_expression = 11;
        state.blob_expression_custom = true;
        state.avatar_random = true;
        state.behavior.cursor_gaze = false;
        state.behavior.music_reactions = false;
        state.music_visual_mode = MusicVisualMode::PetPrimary;
        state.prayer.enabled = false;
        state.prayer.city = 8;
        state.prayer.reminder_minutes = 15;

        let snapshot = PersistedSettings::from_state(&state);
        let mut restored = AppState::default();
        snapshot.apply_to_state(&mut restored);

        assert_eq!(restored.blob_character, 5);
        assert_eq!(restored.blob_palette, 7);
        assert_eq!(restored.blob_expression, 11);
        assert!(restored.blob_expression_custom);
        assert!(restored.avatar_random);
        assert!(!restored.behavior.cursor_gaze);
        assert!(!restored.behavior.music_reactions);
        assert_eq!(restored.music_visual_mode, MusicVisualMode::PetPrimary);
        assert!(!restored.prayer.enabled);
        assert_eq!(restored.prayer.city, 8);
        assert_eq!(restored.prayer.reminder_minutes, 15);
    }
}
