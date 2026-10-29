use crate::models::{
    ActiveModule, BatteryState, IslandView, MediaSource, MediaTrack, MusicVisualMode, TimeInfo,
};
use crate::prayer::PrayerAlert;
use crate::settings::SettingsTab;

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    // Time & periodic
    Tick(TimeInfo),
    
    // Services
    BatteryUpdated(BatteryState),
    MediaUpdated(MediaTrack),
    MediaSourcesUpdated(Vec<MediaSource>),
    SpectrumUpdated([f32; 8]),
    DndUpdated(bool),
    AudioMuteUpdated(bool),
    NotificationReceived,
    PrayerAlert(PrayerAlert),
    
    // Platform & windowing
    WindowOverlapChanged(bool),
    UserHoverChanged(bool),
    OverlapRevealTimeout(u64),
    OverlapDismissCompactTimeout(u64),
    OverlapDismissHideTimeout(u64),
    CollapseCompactTimeout(u64),
    
    // Mode & module transitions
    ViewSwitchRequested(IslandView),
    ModuleSwitchRequested(ActiveModule),
    WelcomeTimeout,
    ScreenUnlocked,
    TriggerWelcome,
    
    // User interactions
    PlayPauseClicked,
    NextTrackClicked,
    PreviousTrackClicked,
    MuteToggleClicked,
    DndToggleClicked,
    ToggleSourceSelector,
    OpenSettings,
    CloseSettings,
    SetSettingsTab(SettingsTab),
    SetCursorGaze(bool),
    SetIdleActing(bool),
    SetClickStunts(bool),
    SetNotificationReactions(bool),
    SetMusicReactions(bool),
    SetSleepyOnLowBattery(bool),
    SetShadowEnabled(bool),
    SetShadowIntensity(u32),
    SetShadowRadius(u32),
    SetShadowOffsetX(i32),
    SetShadowOffsetY(i32),
    SetPrayerEnabled(bool),
    SetPrayerCity(u32),
    SetPrayerReminderMinutes(u32),
    SetMediaVisualMode(MusicVisualMode),
    ResetAvatar,
    ResetSettings,
    SelectMediaSource(String),
    IslandClicked,
    CollapseClicked,
    ToggleSwapPetArt,
    PetClicked,
    PetHappyTimeout(u64),
    PetDizzyTimeout(u64),
    PetPokeReset(u64),
    MochiSlapped,
    MochiAnnoyedTimeout(u64),
    MochiDizzyTimeout(u64),
    MochiHappyTimeout(u64),
    MochiSlapReset(u64),
    CycleBlobCharacter,
    SetBlobCharacter(u32),
    CycleBlobPalette,
    SetBlobPalette(u32),
    CycleBlobExpression,
    SetBlobExpression(u32),
    RandomizeAvatar,
    AvatarRandomTick,
    ToggleAvatarRandom,
    AlertTimeout(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    PlayPause,
    NextTrack,
    PreviousTrack,
    ToggleMute,
    ToggleDnd,
    PersistSettings,
    SelectMediaSource(String),
    ScheduleTimer { id: &'static str, duration_ms: u64, generation: u64 },
    CancelTimer { id: &'static str },
}
