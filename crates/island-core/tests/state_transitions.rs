use island_core::{reduce, ActiveModule, AppEvent, AppState, IslandMode, MediaTrack};

#[test]
fn test_initial_state_wide_welcome() {
    let state = AppState::default();
    assert_eq!(state.mode, IslandMode::Wide);
    assert!(state.is_welcome);
    assert!(state.is_visible);
    assert!(!state.is_overlapping);
    assert_eq!(state.active_module, ActiveModule::System);
}

#[test]
fn test_welcome_timeout_to_compact() {
    let mut state = AppState::default();
    assert!(state.is_welcome);
    assert_eq!(state.mode, IslandMode::Wide);

    reduce(&mut state, AppEvent::WelcomeTimeout);
    assert_eq!(state.mode, IslandMode::Compact);
    assert!(!state.is_welcome);
}

#[test]
fn test_screen_unlocked_triggers_welcome() {
    let mut state = AppState::default();
    // First settle to compact
    reduce(&mut state, AppEvent::WelcomeTimeout);
    assert_eq!(state.mode, IslandMode::Compact);
    assert!(!state.is_welcome);

    // Screen unlocks from lockscreen
    let effects = reduce(&mut state, AppEvent::ScreenUnlocked);
    assert_eq!(state.mode, IslandMode::Wide);
    assert!(state.is_welcome);
    assert!(state.pet_is_happy);
    assert_eq!(effects, vec![island_core::events::Effect::ScheduleTimer {
        id: "welcome_timeout",
        duration_ms: 3500,
        generation: 2,
    }]);

    // Timeout settles it back to compact
    reduce(&mut state, AppEvent::WelcomeTimeout);
    assert_eq!(state.mode, IslandMode::Compact);
    assert!(!state.is_welcome);
}

#[test]
fn test_overlap_hides_notch() {
    let mut state = AppState::default();
    state.mode = IslandMode::Compact;
    
    // Overlapping window arrives
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));
    assert_eq!(state.is_overlapping, true);
    assert_eq!(state.is_visible, false);

    // Overlapping window moves away
    reduce(&mut state, AppEvent::WindowOverlapChanged(false));
    assert_eq!(state.is_overlapping, false);
    assert_eq!(state.is_visible, true);
}

#[test]
fn test_hover_overrides_overlap() {
    let mut state = AppState::default();
    state.mode = IslandMode::Compact;
    
    // Window overlaps
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));
    assert_eq!(state.is_visible, false);

    // User moves mouse over notch edge
    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.is_visible, true);

    // User leaves mouse
    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert_eq!(state.is_visible, false);
}

#[test]
fn test_media_playback_activates_media_module() {
    let mut state = AppState::default();
    state.mode = IslandMode::Compact;

    let track = MediaTrack {
        title: "Starboy".to_string(),
        artist: "The Weeknd".to_string(),
        album: "Starboy".to_string(),
        art_url: String::new(),
        is_playing: true,
        position_ms: 1000,
        duration_ms: 230000,
    };

    reduce(&mut state, AppEvent::MediaUpdated(track));
    assert_eq!(state.active_module, ActiveModule::Media);
}

#[test]
fn test_toggle_source_selector_and_select() {
    let mut state = AppState::default();
    assert!(!state.show_source_selector);

    // Toggle on
    reduce(&mut state, AppEvent::ToggleSourceSelector);
    assert!(state.show_source_selector);
    assert_eq!(state.active_module, ActiveModule::Media);

    // Populate sources
    let sources = vec![
        island_core::models::MediaSource {
            id: "org.mpris.MediaPlayer2.spotify".to_string(),
            name: "Spotify".to_string(),
            is_active: false,
            is_playing: true,
        },
        island_core::models::MediaSource {
            id: "org.mpris.MediaPlayer2.plasma-browser-integration".to_string(),
            name: "Zen Browser".to_string(),
            is_active: false,
            is_playing: false,
        },
    ];
    reduce(&mut state, AppEvent::MediaSourcesUpdated(sources));
    assert_eq!(state.media_sources.len(), 2);

    // Select Spotify
    let effects = reduce(&mut state, AppEvent::SelectMediaSource("org.mpris.MediaPlayer2.spotify".to_string()));
    assert_eq!(effects, vec![island_core::events::Effect::SelectMediaSource("org.mpris.MediaPlayer2.spotify".to_string())]);
    assert!(state.media_sources[0].is_active);
    assert!(!state.media_sources[1].is_active);

    // Toggle off
    reduce(&mut state, AppEvent::ToggleSourceSelector);
    assert!(!state.show_source_selector);
}

#[test]
fn test_pet_clicked_and_happy_timeout() {
    let mut state = AppState::default();
    assert!(!state.pet_is_happy);

    // Click pet
    let effects = reduce(&mut state, AppEvent::PetClicked);
    assert!(state.pet_is_happy);
    assert_eq!(state.pet_happy_generation, 1);
    assert_eq!(effects, vec![island_core::events::Effect::ScheduleTimer {
        id: "pet_happy_timeout",
        duration_ms: 1500,
        generation: 1,
    }]);

    // Another click updates generation
    let effects2 = reduce(&mut state, AppEvent::PetClicked);
    assert!(state.pet_is_happy);
    assert_eq!(state.pet_happy_generation, 2);
    assert_eq!(effects2, vec![island_core::events::Effect::ScheduleTimer {
        id: "pet_happy_timeout",
        duration_ms: 1500,
        generation: 2,
    }]);

    // Stale timeout generation 1 does not reset happy state
    reduce(&mut state, AppEvent::PetHappyTimeout(1));
    assert!(state.pet_is_happy);

    // Matching timeout generation 2 resets happy state
    reduce(&mut state, AppEvent::PetHappyTimeout(2));
    assert!(!state.pet_is_happy);
}
