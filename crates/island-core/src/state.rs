use crate::models::{
    ActiveModule, AlertKind, AudioSpectrum, BatteryState, IslandView, MediaSource, MediaTrack,
    MusicVisualMode, TimeInfo,
};
use crate::prayer::{PrayerAlert, PrayerSettings};
use crate::settings::{PetBehaviorSettings, SettingsTab};
use crate::view_engine::{SlotContext, ViewEngine};

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
pub struct AppState {
    pub engine: ViewEngine,
    pub mode: IslandView,
    pub previous_mode: IslandView,
    pub active_module: ActiveModule,
    pub is_visible: bool,
    pub is_overlapping: bool,
    pub is_hovered: bool,

    // Overlap transition sequencing
    pub overlap_reveal_generation: u64,
    pub overlap_dismiss_generation: u64,

    // Services data
    pub time: TimeInfo,
    pub battery: BatteryState,
    pub media: MediaTrack,
    pub spectrum: AudioSpectrum,
    
    // Actions toggles
    pub is_muted: bool,
    pub is_dnd: bool,

    // Media sources selection
    pub media_sources: Vec<MediaSource>,
    pub show_source_selector: bool,
    pub settings_open: bool,
    pub settings_tab: SettingsTab,
    pub behavior: PetBehaviorSettings,
    pub prayer: PrayerSettings,

    // Presentation states
    pub alert_kind: AlertKind,
    pub alert_generation: u64,
    pub prayer_alert: Option<PrayerAlert>,
    pub notification_generation: u64,
    pub music_visual_mode: MusicVisualMode,
    pub pet_is_happy: bool,
    pub pet_happy_generation: u64,
    pub pet_is_dizzy: bool,
    pub pet_is_sleepy: bool,
    pub pet_poke_count: u32,
    pub pet_dizzy_generation: u64,

    // Coucou Mochi states
    pub mochi_mood: MochiMoodState,
    pub mochi_slap_count: u32,
    pub mochi_generation: u64,

    // Bloub Pet states
    pub blob_character: u32,
    pub blob_palette: u32,
    pub blob_expression: u32,
    pub blob_expression_custom: bool,
    pub avatar_random: bool,
    pub avatar_seed: u64,

    // Welcome Greeting states
    pub is_welcome: bool,
    pub welcome_generation: u64,
    pub welcome_title: String,
    pub welcome_subtitle: String,
}

impl AppState {
    pub fn sync_engine(&mut self) {
        self.mode = self.engine.current_view;
        self.previous_mode = self.engine.previous_view;
        self.active_module = self.engine.active_module;
        self.is_visible = self.engine.is_visible;
        self.is_overlapping = self.engine.is_overlapping;
        self.is_hovered = self.engine.is_hovered;
    }

    pub fn slot_context(&self) -> SlotContext<'_> {
        SlotContext {
            time: &self.time,
            battery: &self.battery,
            media: &self.media,
            view: self.engine.current_view,
            is_muted: self.is_muted,
            is_dnd: self.is_dnd,
            alert_kind: self.alert_kind,
        }
    }

    pub fn update_active_module(&mut self) {
        let ctx = SlotContext {
            time: &self.time,
            battery: &self.battery,
            media: &self.media,
            view: self.engine.current_view,
            is_muted: self.is_muted,
            is_dnd: self.is_dnd,
            alert_kind: self.alert_kind,
        };
        self.engine.resolve_active_module(&ctx);
        self.sync_engine();
    }
}

impl Default for AppState {
    fn default() -> Self {
        let engine = ViewEngine::new();
        Self {
            mode: engine.current_view,
            previous_mode: engine.previous_view,
            active_module: engine.active_module,
            is_visible: engine.is_visible,
            is_overlapping: engine.is_overlapping,
            is_hovered: engine.is_hovered,
            overlap_reveal_generation: 0,
            overlap_dismiss_generation: 0,
            engine,
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
            show_source_selector: false,
            settings_open: false,
            settings_tab: SettingsTab::Avatar,
            behavior: PetBehaviorSettings::default(),
            prayer: PrayerSettings::default(),
            alert_kind: AlertKind::None,
            alert_generation: 0,
            prayer_alert: None,
            notification_generation: 0,
            music_visual_mode: MusicVisualMode::ArtworkPrimary,
            pet_is_happy: false,
            pet_happy_generation: 0,
            pet_is_dizzy: false,
            pet_is_sleepy: false,
            pet_poke_count: 0,
            pet_dizzy_generation: 0,
            mochi_mood: MochiMoodState::Idle,
            mochi_slap_count: 0,
            mochi_generation: 0,
            blob_character: 1, // 1 = Galet
            blob_palette: 0, // 0 = notchde light
            blob_expression: 0, // 0 = Neutre
            blob_expression_custom: false,
            avatar_random: false,
            avatar_seed: 0x5eed_1234_abcd_9876,
            is_welcome: true,
            welcome_generation: 1,
            welcome_title: "Welcome back!".to_string(),
            welcome_subtitle: "Have a wonderful day ✨".to_string(),
        }
    }
}
