use crate::models::{ActiveModule, BatteryState, IslandView, MediaTrack, TimeInfo};

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    // Time & periodic
    Tick(TimeInfo),
    
    // Services
    BatteryUpdated(BatteryState),
    MediaUpdated(MediaTrack),
    SpectrumUpdated([f32; 8]),
    DndUpdated(bool),
    AudioMuteUpdated(bool),
    
    // Platform & windowing
    WindowOverlapChanged(bool),
    UserHoverChanged(bool),
    
    // Mode & module transitions
    ViewSwitchRequested(IslandView),
    ModuleSwitchRequested(ActiveModule),
    WelcomeTimeout,
    
    // User interactions
    PlayPauseClicked,
    NextTrackClicked,
    PreviousTrackClicked,
    MuteToggleClicked,
    DndToggleClicked,
    IslandClicked,
    CollapseClicked,
    ToggleSwapPetArt,
    AlertTimeout(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    PlayPause,
    NextTrack,
    PreviousTrack,
    ToggleMute,
    ToggleDnd,
    ScheduleTimer { id: &'static str, duration_ms: u64, generation: u64 },
    CancelTimer { id: &'static str },
}
