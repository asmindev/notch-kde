use crate::models::{
    ActiveModule, AlertKind, AudioSpectrum, BatteryState, IslandView, MediaTrack, MusicVisualMode,
    TimeInfo,
};
use crate::view_engine::{SlotContext, ViewEngine};

#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub engine: ViewEngine,
    pub mode: IslandView,
    pub previous_mode: IslandView,
    pub active_module: ActiveModule,
    pub is_visible: bool,
    pub is_overlapping: bool,
    pub is_hovered: bool,
    
    // Services data
    pub time: TimeInfo,
    pub battery: BatteryState,
    pub media: MediaTrack,
    pub spectrum: AudioSpectrum,
    
    // Actions toggles
    pub is_muted: bool,
    pub is_dnd: bool,

    // Presentation states
    pub alert_kind: AlertKind,
    pub alert_generation: u64,
    pub music_visual_mode: MusicVisualMode,
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
            alert_kind: AlertKind::None,
            alert_generation: 0,
            music_visual_mode: MusicVisualMode::ArtworkPrimary,
        }
    }
}
