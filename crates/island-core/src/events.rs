use crate::models::{
    ActiveModule, BatteryState, IslandView, MediaSource, MediaTrack, MusicVisualMode, TimeInfo,
};
use crate::prayer::PrayerAlert;
use crate::settings::{CompactModeStyle, PetRenderer, SettingsTab};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimerId {
    Alert,
    CollapseCompact,
    MochiAnnoyed,
    MochiDizzy,
    MochiSlapReset,
    OverlapDismissCompact,
    OverlapDismissHide,
    OverlapReveal,
    PetDizzy,
    PetHappy,
    PetPokeReset,
    Welcome,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    // Time & periodic
    Tick(TimeInfo),
    TimerFired { id: TimerId, generation: u64 },
    Shutdown,

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

    // Mode & module transitions
    ViewSwitchRequested(IslandView),
    ModuleSwitchRequested(ActiveModule),
    ScreenUnlocked,
    TriggerWelcome,

    // User interactions
    PlayPauseClicked,
    NextTrackClicked,
    PreviousTrackClicked,
    MuteToggleClicked,
    DndToggleClicked,
    ToggleSourceSelector,
    TogglePrayerPanel,
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
    SetPetRenderer(PetRenderer),
    SetCompactModeStyle(CompactModeStyle),
    ResetAvatar,
    ResetSettings,
    SelectMediaSource(String),
    IslandClicked,
    CollapseClicked,
    ToggleSwapPetArt,
    PetClicked,
    MochiSlapped,
    CycleBlobCharacter,
    SetBlobCharacter(u32),
    CycleBlobPalette,
    SetBlobPalette(u32),
    CycleBlobExpression,
    SetBlobExpression(u32),
    RandomizeAvatar,
    AvatarRandomTick,
    ToggleAvatarRandom,
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
    ScheduleTimer {
        id: TimerId,
        duration_ms: u64,
        generation: u64,
    },
}
