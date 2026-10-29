use chrono::Utc;
use island_core::prayer::{Prayer, PrayerAlert, PrayerAlertKind};
use island_core::{ActiveModule, AppEvent, AppState, BatteryState, IslandMode, MediaTrack, reduce};

#[test]
fn test_default_slot_is_system() {
    let state = AppState::default();
    assert_eq!(state.active_module, ActiveModule::System);
}

#[test]
fn test_media_playing_takes_priority_over_system() {
    let mut state = AppState::default();
    assert_eq!(state.active_module, ActiveModule::System);

    // Media starts playing -> Priority::HIGH (100) > System::LOW (10)
    reduce(
        &mut state,
        AppEvent::MediaUpdated(MediaTrack {
            title: "Starboy".to_string(),
            artist: "The Weeknd".to_string(),
            album: "Starboy".to_string(),
            art_url: String::new(),
            is_playing: true,
            position_ms: 0,
            duration_ms: 200000,
        }),
    );
    assert_eq!(state.active_module, ActiveModule::Media);

    // Media stopped (no title, not playing) -> priority drops to INACTIVE, falls back to System
    reduce(&mut state, AppEvent::MediaUpdated(MediaTrack::default()));
    assert_eq!(state.active_module, ActiveModule::System);
}

fn paused_track() -> MediaTrack {
    MediaTrack {
        title: "Paused Song".to_string(),
        artist: "Artist".to_string(),
        album: "Album".to_string(),
        art_url: String::new(),
        is_playing: false,
        position_ms: 1000,
        duration_ms: 200000,
    }
}

#[test]
fn test_paused_media_stays_in_compact_but_falls_back_in_wide() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::WelcomeTimeout);
    assert_eq!(state.mode, IslandMode::Compact);

    reduce(&mut state, AppEvent::MediaUpdated(paused_track()));
    assert_eq!(state.active_module, ActiveModule::Media);

    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.mode, IslandMode::Wide);
    assert_eq!(state.active_module, ActiveModule::System);

    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert_eq!(state.mode, IslandMode::Compact);
    assert_eq!(state.active_module, ActiveModule::Media);
}

#[test]
fn test_paused_media_falls_back_in_expand() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::WelcomeTimeout);
    reduce(&mut state, AppEvent::MediaUpdated(paused_track()));
    assert_eq!(state.active_module, ActiveModule::Media);

    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.mode, IslandMode::Expand);
    assert_eq!(state.active_module, ActiveModule::System);

    reduce(&mut state, AppEvent::CollapseClicked);
    let generation = state.overlap_dismiss_generation;
    reduce(&mut state, AppEvent::CollapseCompactTimeout(generation));
    assert_eq!(state.mode, IslandMode::Compact);
    assert_eq!(state.active_module, ActiveModule::Media);
}

#[test]
fn test_source_selector_keeps_paused_media_active_in_wide() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::WelcomeTimeout);
    reduce(&mut state, AppEvent::MediaUpdated(paused_track()));
    assert_eq!(state.mode, IslandMode::Compact);
    assert_eq!(state.active_module, ActiveModule::Media);

    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.mode, IslandMode::Wide);
    assert_eq!(state.active_module, ActiveModule::System);

    reduce(&mut state, AppEvent::ToggleSourceSelector);
    assert!(state.show_source_selector);
    assert_eq!(state.active_module, ActiveModule::Media);

    reduce(&mut state, AppEvent::ToggleSourceSelector);
    assert!(!state.show_source_selector);
    assert_eq!(state.active_module, ActiveModule::System);

    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert_eq!(state.mode, IslandMode::Compact);
    assert_eq!(state.active_module, ActiveModule::Media);
}

#[test]
fn test_playing_media_stays_active_in_all_views() {
    let mut state = AppState::default();
    let mut track = paused_track();
    track.is_playing = true;

    reduce(&mut state, AppEvent::MediaUpdated(track.clone()));
    assert_eq!(state.mode, IslandMode::Wide);
    assert_eq!(state.active_module, ActiveModule::Media);

    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.mode, IslandMode::Expand);
    assert_eq!(state.active_module, ActiveModule::Media);

    reduce(&mut state, AppEvent::CollapseClicked);
    let generation = state.overlap_dismiss_generation;
    reduce(&mut state, AppEvent::CollapseCompactTimeout(generation));
    assert_eq!(state.mode, IslandMode::Compact);
    assert_eq!(state.active_module, ActiveModule::Media);
}

#[test]
fn test_critical_battery_takes_priority_over_media() {
    let mut state = AppState::default();

    // 1. Media is playing -> ActiveModule::Media
    reduce(
        &mut state,
        AppEvent::MediaUpdated(MediaTrack {
            title: "Blinding Lights".to_string(),
            artist: "The Weeknd".to_string(),
            album: "After Hours".to_string(),
            art_url: String::new(),
            is_playing: true,
            position_ms: 0,
            duration_ms: 200000,
        }),
    );
    assert_eq!(state.active_module, ActiveModule::Media);

    // 2. Battery drops to 8% discharging -> SystemModule emits Priority::CRITICAL (300) > Media::HIGH (100)!
    reduce(
        &mut state,
        AppEvent::BatteryUpdated(BatteryState {
            percentage: 8,
            is_charging: false,
        }),
    );
    assert_eq!(state.active_module, ActiveModule::System);

    // 3. Charger plugged in -> Triggers alert, and after alert times out Media re-takes slot!
    let effects = reduce(
        &mut state,
        AppEvent::BatteryUpdated(BatteryState {
            percentage: 8,
            is_charging: true,
        }),
    );
    // Charger connected triggers global AlertModule (Priority::URGENT 200) and 2.8s timer with generation ID
    assert_eq!(state.active_module, ActiveModule::Alert);
    assert_eq!(state.alert_generation, 1);
    assert!(effects.contains(&island_core::Effect::ScheduleTimer {
        id: "alert_timeout",
        duration_ms: 2800,
        generation: 1,
    }));

    // Timeout occurs with matching generation -> Alert dismissed, falls back to Media (Priority::HIGH 100)
    reduce(&mut state, AppEvent::AlertTimeout(1));
    assert_eq!(state.active_module, ActiveModule::Media);
}

#[test]
fn test_alert_generation_safety() {
    let mut state = AppState::default();

    // 1. Charger connected -> Alert Gen 1
    reduce(
        &mut state,
        AppEvent::BatteryUpdated(BatteryState {
            percentage: 50,
            is_charging: true,
        }),
    );
    assert_eq!(state.alert_generation, 1);
    assert_eq!(state.active_module, ActiveModule::Alert);

    // 2. Charger unplugged and plugged back in -> Alert Gen 2
    reduce(
        &mut state,
        AppEvent::BatteryUpdated(BatteryState {
            percentage: 50,
            is_charging: false,
        }),
    );
    reduce(
        &mut state,
        AppEvent::BatteryUpdated(BatteryState {
            percentage: 50,
            is_charging: true,
        }),
    );
    assert_eq!(state.alert_generation, 2);
    assert_eq!(state.active_module, ActiveModule::Alert);

    // 3. Stale timer from Gen 1 arrives -> Ignored, Alert remains active!
    reduce(&mut state, AppEvent::AlertTimeout(1));
    assert_eq!(state.alert_generation, 2);
    assert_eq!(state.active_module, ActiveModule::Alert); // Still alert!

    // 4. Timer for Gen 2 arrives -> Properly dismissed!
    reduce(&mut state, AppEvent::AlertTimeout(2));
    assert_eq!(state.active_module, ActiveModule::System);
}

#[test]
fn test_toggle_swap_pet_art_enum() {
    use island_core::models::MusicVisualMode;
    let mut state = AppState::default();
    assert_eq!(state.music_visual_mode, MusicVisualMode::ArtworkPrimary);

    reduce(&mut state, AppEvent::ToggleSwapPetArt);
    assert_eq!(state.music_visual_mode, MusicVisualMode::PetPrimary);

    reduce(&mut state, AppEvent::ToggleSwapPetArt);
    assert_eq!(state.music_visual_mode, MusicVisualMode::ArtworkPrimary);
}

#[test]
fn test_dnd_and_audio_mute_events() {
    let mut state = AppState::default();
    assert!(!state.is_dnd);
    assert!(!state.is_muted);

    // Event-driven DND update
    reduce(&mut state, AppEvent::DndUpdated(true));
    assert!(state.is_dnd);

    reduce(&mut state, AppEvent::DndUpdated(false));
    assert!(!state.is_dnd);

    // Event-driven Audio mute update
    reduce(&mut state, AppEvent::AudioMuteUpdated(true));
    assert!(state.is_muted);

    reduce(&mut state, AppEvent::AudioMuteUpdated(false));
    assert!(!state.is_muted);

    // Toggle clicks emit side effects and update state
    let effects = reduce(&mut state, AppEvent::MuteToggleClicked);
    assert!(state.is_muted);
    assert!(effects.contains(&island_core::Effect::ToggleMute));

    let effects = reduce(&mut state, AppEvent::DndToggleClicked);
    assert!(state.is_dnd);
    assert!(effects.contains(&island_core::Effect::ToggleDnd));
}

fn settle_wide(state: &mut AppState) {
    reduce(state, AppEvent::WelcomeTimeout);
    reduce(state, AppEvent::UserHoverChanged(true));
}

fn playing_track() -> MediaTrack {
    MediaTrack {
        title: "Starboy".to_string(),
        artist: "The Weeknd".to_string(),
        album: "Starboy".to_string(),
        art_url: String::new(),
        is_playing: true,
        position_ms: 0,
        duration_ms: 200000,
    }
}

fn prayer_alert() -> PrayerAlert {
    PrayerAlert {
        prayer: Prayer::Dhuhr,
        kind: PrayerAlertKind::Adhan,
        at: Utc::now(),
        title: "Dzuhur".to_string(),
        time_label: "12:00".to_string(),
        city_label: "Jakarta · WIB".to_string(),
    }
}

#[test]
fn test_prayer_panel_takes_priority_over_playing_media_in_wide() {
    let mut state = AppState::default();
    settle_wide(&mut state);
    reduce(&mut state, AppEvent::MediaUpdated(playing_track()));
    assert_eq!(state.active_module, ActiveModule::Media);

    reduce(&mut state, AppEvent::TogglePrayerPanel);
    assert!(state.prayer_panel_open);
    assert_eq!(state.active_module, ActiveModule::Prayer);

    reduce(&mut state, AppEvent::TogglePrayerPanel);
    assert!(!state.prayer_panel_open);
    assert_eq!(state.active_module, ActiveModule::Media);
}

#[test]
fn test_alert_overrides_prayer_and_returns_to_prayer() {
    let mut state = AppState::default();
    settle_wide(&mut state);
    reduce(&mut state, AppEvent::TogglePrayerPanel);
    assert_eq!(state.active_module, ActiveModule::Prayer);

    reduce(&mut state, AppEvent::PrayerAlert(prayer_alert()));
    assert_eq!(state.active_module, ActiveModule::Alert);

    let generation = state.alert_generation;
    reduce(&mut state, AppEvent::AlertTimeout(generation));
    assert_eq!(state.active_module, ActiveModule::Prayer);
}

#[test]
fn test_prayer_panel_restores_after_compact_and_is_kept_in_expand() {
    let mut state = AppState::default();
    settle_wide(&mut state);
    reduce(&mut state, AppEvent::TogglePrayerPanel);
    assert_eq!(state.active_module, ActiveModule::Prayer);

    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert_eq!(state.mode, IslandMode::Compact);
    assert_eq!(state.active_module, ActiveModule::System);
    assert!(state.prayer_panel_open);

    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.active_module, ActiveModule::Prayer);

    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.mode, IslandMode::Expand);
    assert_eq!(state.active_module, ActiveModule::Prayer);
}

#[test]
fn test_prayer_panel_and_source_selector_are_mutually_exclusive() {
    let mut state = AppState::default();
    settle_wide(&mut state);

    reduce(&mut state, AppEvent::TogglePrayerPanel);
    assert!(state.prayer_panel_open);
    assert!(!state.show_source_selector);

    reduce(&mut state, AppEvent::ToggleSourceSelector);
    assert!(!state.prayer_panel_open);
    assert!(state.show_source_selector);
    assert_eq!(state.active_module, ActiveModule::Media);

    reduce(&mut state, AppEvent::TogglePrayerPanel);
    assert!(state.prayer_panel_open);
    assert!(!state.show_source_selector);
    assert_eq!(state.active_module, ActiveModule::Prayer);
}

#[test]
fn test_module_switch_request_cannot_bypass_resolver() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::WelcomeTimeout);

    reduce(
        &mut state,
        AppEvent::ModuleSwitchRequested(ActiveModule::Prayer),
    );
    assert_eq!(state.active_module, ActiveModule::System);

    let mut playing = paused_track();
    playing.is_playing = true;
    reduce(&mut state, AppEvent::MediaUpdated(playing));
    assert_eq!(state.active_module, ActiveModule::Media);

    reduce(
        &mut state,
        AppEvent::ModuleSwitchRequested(ActiveModule::System),
    );
    assert_eq!(state.active_module, ActiveModule::Media);
}
