use crate::models::{
    AlertKind, BatteryState, IslandView, MediaSource, MediaTrack, MusicVisualMode, TimeInfo,
};
use crate::module::ModuleId;
use crate::prayer::{PrayerAlert, PrayerSettings};
use crate::settings::{PetBehaviorSettings, SettingsTab, ShadowSettings};
use crate::state::{AppState, MochiMoodState};

#[derive(Debug, Clone, PartialEq)]
pub struct NavigationSnapshot {
    pub current_view: IslandView,
    pub previous_view: IslandView,
    pub active_module: ModuleId,
    pub is_visible: bool,
    pub is_overlapping: bool,
    pub is_hovered: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DomainSnapshot {
    pub time: TimeInfo,
    pub battery: BatteryState,
    pub media: MediaTrack,
    pub spectrum: [f32; 8],
    pub is_muted: bool,
    pub is_dnd: bool,
    pub media_sources: Vec<MediaSource>,
    pub alert_kind: AlertKind,
    pub prayer_alert: Option<PrayerAlert>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PresentationSnapshot {
    pub settings_open: bool,
    pub settings_tab: SettingsTab,
    pub show_source_selector: bool,
    pub prayer_panel_open: bool,
    pub notification_generation: u64,
    pub pet_is_happy: bool,
    pub pet_happy_generation: u64,
    pub pet_is_dizzy: bool,
    pub pet_is_sleepy: bool,
    pub pet_poke_count: u32,
    pub pet_dizzy_generation: u64,
    pub mochi_mood: MochiMoodState,
    pub mochi_slap_count: u32,
    pub mochi_generation: u64,
    pub is_welcome: bool,
    pub welcome_generation: u64,
    pub welcome_title: String,
    pub welcome_subtitle: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsSnapshot {
    pub behavior: PetBehaviorSettings,
    pub shadow: ShadowSettings,
    pub prayer: PrayerSettings,
    pub music_visual_mode: MusicVisualMode,
    pub blob_character: u32,
    pub blob_palette: u32,
    pub blob_expression: u32,
    pub blob_expression_custom: bool,
    pub avatar_random: bool,
    pub avatar_seed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewSnapshot {
    pub navigation: NavigationSnapshot,
    pub domain: DomainSnapshot,
    pub presentation: PresentationSnapshot,
    pub settings: SettingsSnapshot,
}

impl ViewSnapshot {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            navigation: NavigationSnapshot {
                current_view: state.view.mode(),
                previous_view: state.view.previous_mode(),
                active_module: state.view.active_module(),
                is_visible: state.view.is_visible(),
                is_overlapping: state.view.is_overlapping(),
                is_hovered: state.view.is_hovered(),
            },
            domain: DomainSnapshot {
                time: state.domain.time.clone(),
                battery: state.domain.battery.clone(),
                media: state.domain.media.clone(),
                spectrum: state.domain.spectrum.bars,
                is_muted: state.domain.is_muted,
                is_dnd: state.domain.is_dnd,
                media_sources: state.domain.media_sources.clone(),
                alert_kind: state.domain.alert_kind,
                prayer_alert: state.domain.prayer_alert.clone(),
            },
            presentation: PresentationSnapshot {
                settings_open: state.presentation.settings_open,
                settings_tab: state.presentation.settings_tab,
                show_source_selector: state.module.show_source_selector,
                prayer_panel_open: state.module.prayer_panel_open,
                notification_generation: state.presentation.notification_generation,
                pet_is_happy: state.presentation.pet_is_happy,
                pet_happy_generation: state.presentation.pet_happy_generation,
                pet_is_dizzy: state.presentation.pet_is_dizzy,
                pet_is_sleepy: state.presentation.pet_is_sleepy,
                pet_poke_count: state.presentation.pet_poke_count,
                pet_dizzy_generation: state.presentation.pet_dizzy_generation,
                mochi_mood: state.presentation.mochi_mood,
                mochi_slap_count: state.presentation.mochi_slap_count,
                mochi_generation: state.presentation.mochi_generation,
                is_welcome: state.presentation.is_welcome,
                welcome_generation: state.presentation.welcome_generation,
                welcome_title: state.presentation.welcome_title.clone(),
                welcome_subtitle: state.presentation.welcome_subtitle.clone(),
            },
            settings: SettingsSnapshot {
                behavior: state.settings.behavior,
                shadow: state.settings.shadow,
                prayer: state.settings.prayer,
                music_visual_mode: state.settings.music_visual_mode,
                blob_character: state.settings.blob_character,
                blob_palette: state.settings.blob_palette,
                blob_expression: state.settings.blob_expression,
                blob_expression_custom: state.settings.blob_expression_custom,
                avatar_random: state.settings.avatar_random,
                avatar_seed: state.settings.avatar_seed,
            },
        }
    }
}

impl AppState {
    pub fn snapshot(&self) -> ViewSnapshot {
        ViewSnapshot::from_state(self)
    }
}

impl From<&AppState> for ViewSnapshot {
    fn from(state: &AppState) -> Self {
        Self::from_state(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_stable_and_tracks_state_changes() {
        let mut state = AppState::default();
        state.domain.media.title = "Song".to_string();
        state.view.engine.set_view(IslandView::Compact);
        state.module.show_source_selector = true;
        state.settings.blob_character = 4;
        state.presentation.is_welcome = false;
        state.update_active_module();

        let first = state.snapshot();
        let second = state.snapshot();
        assert_eq!(first, second);
        assert_eq!(first.navigation.current_view, IslandView::Compact);
        assert_eq!(first.navigation.active_module, ModuleId::MEDIA);
        assert_eq!(first.domain.media.title, "Song");
        assert!(first.presentation.show_source_selector);
        assert_eq!(first.settings.blob_character, 4);
        assert!(!first.presentation.is_welcome);

        state.domain.media.title = "Other".to_string();
        assert_ne!(first, state.snapshot());
    }
}
