use island_core::{reduce, ActiveModule, AppEvent, AppState, IslandMode, MediaTrack};

#[test]
fn test_initial_state_compact() {
    let state = AppState::default();
    assert_eq!(state.mode, IslandMode::Compact);
    assert!(state.is_visible);
    assert!(!state.is_overlapping);
    assert_eq!(state.active_module, ActiveModule::System);
}

#[test]
fn test_welcome_timeout_to_compact() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::WelcomeTimeout);
    assert_eq!(state.mode, IslandMode::Compact);
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
