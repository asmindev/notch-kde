use crate::content_resolver::resolve;
use crate::models::{
    AlertKind, AudioSpectrum, BatteryState, IslandView, MediaSource, MediaTrack, MusicVisualMode,
    TimeInfo,
};
use crate::module::{ModuleContext, ModuleId};
use crate::prayer::{PrayerAlert, PrayerSettings};
use crate::settings::{PetBehaviorSettings, SettingsTab, ShadowSettings};
use crate::view_engine::ViewEngine;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum MochiMoodState {
    #[default]
    Idle = 0,
    Happy = 1,
    Annoyed = 2,
    Dizzy = 3,
    Dancing = 4,
    Sleeping = 5,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    pub engine: ViewEngine,
    pub mode: IslandView,
    pub previous_mode: IslandView,
    pub active_module: ModuleId,
    pub is_visible: bool,
    pub is_overlapping: bool,
    pub is_hovered: bool,
}

impl ViewState {
    pub fn new() -> Self {
        let engine = ViewEngine::new();
        Self {
            mode: engine.current_view,
            previous_mode: engine.previous_view,
            active_module: engine.active_module,
            is_visible: engine.is_visible,
            is_overlapping: engine.is_overlapping,
            is_hovered: engine.is_hovered,
            engine,
        }
    }

    pub fn sync_from_engine(&mut self) {
        self.mode = self.engine.current_view;
        self.previous_mode = self.engine.previous_view;
        self.active_module = self.engine.active_module;
        self.is_visible = self.engine.is_visible;
        self.is_overlapping = self.engine.is_overlapping;
        self.is_hovered = self.engine.is_hovered;
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DomainState {
    pub time: TimeInfo,
    pub battery: BatteryState,
    pub media: MediaTrack,
    pub spectrum: AudioSpectrum,
    pub is_muted: bool,
    pub is_dnd: bool,
    pub media_sources: Vec<MediaSource>,
    pub alert_kind: AlertKind,
    pub alert_generation: u64,
    pub prayer_alert: Option<PrayerAlert>,
}

impl Default for DomainState {
    fn default() -> Self {
        Self {
            time: TimeInfo {
                time_str: "00:00".to_string(),
                date_str: "".to_string(),
            },
            battery: BatteryState::default(),
            media: MediaTrack::default(),
            spectrum: AudioSpectrum::default(),
            is_muted: false,
            is_dnd: false,
            media_sources: Vec::new(),
            alert_kind: AlertKind::None,
            alert_generation: 0,
            prayer_alert: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModuleState {
    pub show_source_selector: bool,
    pub prayer_panel_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuntimeState {
    pub overlap_reveal_generation: u64,
    pub overlap_dismiss_generation: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserSettings {
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

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            behavior: PetBehaviorSettings::default(),
            shadow: ShadowSettings::default(),
            prayer: PrayerSettings::default(),
            music_visual_mode: MusicVisualMode::ArtworkPrimary,
            blob_character: 1,
            blob_palette: 0,
            blob_expression: 0,
            blob_expression_custom: false,
            avatar_random: false,
            avatar_seed: 0x5eed_1234_abcd_9876,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PresentationState {
    pub settings_open: bool,
    pub settings_tab: SettingsTab,
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

impl Default for PresentationState {
    fn default() -> Self {
        Self {
            settings_open: false,
            settings_tab: SettingsTab::Avatar,
            notification_generation: 0,
            pet_is_happy: false,
            pet_happy_generation: 0,
            pet_is_dizzy: false,
            pet_is_sleepy: false,
            pet_poke_count: 0,
            pet_dizzy_generation: 0,
            mochi_mood: MochiMoodState::Idle,
            mochi_slap_count: 0,
            mochi_generation: 0,
            is_welcome: true,
            welcome_generation: 1,
            welcome_title: "Welcome back!".to_string(),
            welcome_subtitle: "Have a wonderful day ✨".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppState {
    pub view: ViewState,
    pub domain: DomainState,
    pub module: ModuleState,
    pub runtime: RuntimeState,
    pub settings: UserSettings,
    pub presentation: PresentationState,
}

impl AppState {
    pub fn sync_engine(&mut self) {
        self.view.sync_from_engine();
    }

    pub fn module_context(&self) -> ModuleContext<'_> {
        ModuleContext {
            time: &self.domain.time,
            battery: &self.domain.battery,
            media: &self.domain.media,
            view: self.view.engine.current_view,
            is_muted: self.domain.is_muted,
            is_dnd: self.domain.is_dnd,
            alert_kind: self.domain.alert_kind,
            prayer_panel_open: self.module.prayer_panel_open,
            source_selector_open: self.module.show_source_selector,
        }
    }

    pub fn slot_context(&self) -> ModuleContext<'_> {
        self.module_context()
    }

    pub fn update_active_module(&mut self) {
        let resolved = {
            let ctx = self.module_context();
            resolve(
                self.view.engine.registry(),
                &ctx,
                self.view.engine.current_view,
            )
        };
        self.view.engine.apply_resolved(resolved);
        self.sync_engine();
    }
}
