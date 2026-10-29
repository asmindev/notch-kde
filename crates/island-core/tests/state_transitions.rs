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
    assert_eq!(
        effects,
        vec![
            island_core::events::Effect::ScheduleTimer {
                id: "pet_happy_timeout",
                duration_ms: 1500,
                generation: 1,
            },
            island_core::events::Effect::ScheduleTimer {
                id: "welcome_timeout",
                duration_ms: 3500,
                generation: 2,
            },
        ]
    );

    // Timeout settles it back to compact
    reduce(&mut state, AppEvent::WelcomeTimeout);
    assert_eq!(state.mode, IslandMode::Compact);
    assert!(!state.is_welcome);
}

#[test]
fn test_overlap_hides_notch() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::ViewSwitchRequested(IslandMode::Compact));
    
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
    reduce(&mut state, AppEvent::ViewSwitchRequested(IslandMode::Compact));
    
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
    reduce(&mut state, AppEvent::ViewSwitchRequested(IslandMode::Compact));

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
    assert!(effects.contains(&island_core::events::Effect::ScheduleTimer {
        id: "pet_happy_timeout",
        duration_ms: 1500,
        generation: 1,
    }));

    // Another click updates generation
    let effects2 = reduce(&mut state, AppEvent::PetClicked);
    assert!(state.pet_is_happy);
    assert_eq!(state.pet_happy_generation, 2);
    assert!(effects2.contains(&island_core::events::Effect::ScheduleTimer {
        id: "pet_happy_timeout",
        duration_ms: 1500,
        generation: 2,
    }));

    // Stale timeout generation 1 does not reset happy state
    reduce(&mut state, AppEvent::PetHappyTimeout(1));
    assert!(state.pet_is_happy);

    // Matching timeout generation 2 resets happy state
    reduce(&mut state, AppEvent::PetHappyTimeout(2));
    assert!(!state.pet_is_happy);
}

#[test]
fn test_pet_rapid_poking_triggers_dizzy() {
    let mut state = AppState::default();
    assert!(!state.pet_is_dizzy);
    assert_eq!(state.mochi_mood, island_core::state::MochiMoodState::Idle);

    // Poke 1, 2 times -> Annoyed (slaps < 3)
    reduce(&mut state, AppEvent::PetClicked);
    assert_eq!(state.mochi_mood, island_core::state::MochiMoodState::Annoyed);
    reduce(&mut state, AppEvent::PetClicked);
    assert_eq!(state.mochi_mood, island_core::state::MochiMoodState::Annoyed);
    assert!(!state.pet_is_dizzy);

    // Poke 3rd time -> triggers dizzy (Coucou 3-slap mechanic)!
    let effects = reduce(&mut state, AppEvent::PetClicked);
    assert!(state.pet_is_dizzy);
    assert_eq!(state.mochi_mood, island_core::state::MochiMoodState::Dizzy);
    assert!(!state.pet_is_happy);
    assert!(effects.contains(&island_core::events::Effect::ScheduleTimer {
        id: "mochi_dizzy_timeout",
        duration_ms: 2200,
        generation: state.mochi_generation,
    }));

    // Dizzy timeout resets dizzy state
    let generation = state.mochi_generation;
    reduce(&mut state, AppEvent::MochiDizzyTimeout(generation));
    assert!(!state.pet_is_dizzy);
    assert_eq!(state.mochi_mood, island_core::state::MochiMoodState::Idle);
    assert_eq!(state.mochi_slap_count, 0);
}

#[test]
fn test_pet_sleepy_on_low_battery() {
    let mut state = AppState::default();
    assert!(!state.pet_is_sleepy);
    assert_eq!(state.mochi_mood, island_core::state::MochiMoodState::Idle);

    // Battery drops to 15% and not charging -> sleepy
    reduce(&mut state, AppEvent::BatteryUpdated(island_core::models::BatteryState {
        percentage: 15,
        is_charging: false,
    }));
    assert!(state.pet_is_sleepy);
    assert_eq!(state.mochi_mood, island_core::state::MochiMoodState::Sleeping);

    // Plug in charger -> wakes up!
    reduce(&mut state, AppEvent::BatteryUpdated(island_core::models::BatteryState {
        percentage: 15,
        is_charging: true,
    }));
    assert!(!state.pet_is_sleepy);
    assert_eq!(state.mochi_mood, island_core::state::MochiMoodState::Idle);
}

#[test]
fn test_bloub_shape_and_palette_selection_and_cycle() {
    let mut state = AppState::default();
    assert_eq!(state.blob_character, 1); // Default Galet
    assert_eq!(state.blob_palette, 0); // Default notchde light

    // Cycle through all eight shapes: 1 -> 2 -> ... -> 7 -> 0 -> 1
    for expected in [2, 3, 4, 5, 6, 7, 0, 1] {
        reduce(&mut state, AppEvent::CycleBlobCharacter);
        assert_eq!(state.blob_character, expected);
    }

    // Cycle through all fourteen palettes: 0 -> ... -> 13 -> 0
    for expected in 1..14 {
        reduce(&mut state, AppEvent::CycleBlobPalette);
        assert_eq!(state.blob_palette, expected);
    }
    reduce(&mut state, AppEvent::CycleBlobPalette);
    assert_eq!(state.blob_palette, 0);

    // Direct set wraps
    reduce(&mut state, AppEvent::SetBlobCharacter(9));
    assert_eq!(state.blob_character, 1);
    reduce(&mut state, AppEvent::SetBlobPalette(15));
    assert_eq!(state.blob_palette, 1);

    // Expression cycling and direct set wrap through all expressions and mark
    // the choice as an explicit custom override.
    assert_eq!(state.blob_expression, 0);
    assert!(!state.blob_expression_custom);
    for expected in 1..17 {
        reduce(&mut state, AppEvent::CycleBlobExpression);
        assert_eq!(state.blob_expression, expected);
    }
    reduce(&mut state, AppEvent::CycleBlobExpression);
    assert_eq!(state.blob_expression, 0);
    assert!(state.blob_expression_custom);
    reduce(&mut state, AppEvent::SetBlobExpression(18));
    assert_eq!(state.blob_expression, 1);
    assert!(state.blob_expression_custom);

    // One-shot shuffle always changes the avatar, auto-random only when enabled.
    let before = (state.blob_character, state.blob_palette, state.blob_expression);
    reduce(&mut state, AppEvent::RandomizeAvatar);
    assert!(
        (state.blob_character, state.blob_palette, state.blob_expression) != before
    );
    let before_auto = (state.blob_character, state.blob_palette, state.blob_expression);
    reduce(&mut state, AppEvent::AvatarRandomTick);
    assert_eq!(
        (state.blob_character, state.blob_palette, state.blob_expression),
        before_auto
    );
    reduce(&mut state, AppEvent::ToggleAvatarRandom);
    assert!(state.avatar_random);
    assert!(
        (state.blob_character, state.blob_palette, state.blob_expression) != before_auto
    );
}

#[test]
fn notification_event_bumps_generation() {
    let mut state = AppState::default();
    assert_eq!(state.notification_generation, 0);
    reduce(&mut state, AppEvent::NotificationReceived);
    reduce(&mut state, AppEvent::NotificationReceived);
    assert_eq!(state.notification_generation, 2);
}

#[test]
fn test_settings_events_open_tab_toggle_and_persist() {
    let mut state = AppState::default();

    reduce(&mut state, AppEvent::OpenSettings);
    assert!(state.settings_open);

    reduce(&mut state, AppEvent::SetSettingsTab(island_core::SettingsTab::System));
    assert_eq!(state.settings_tab, island_core::SettingsTab::System);

    let effects = reduce(&mut state, AppEvent::SetMusicReactions(false));
    assert!(!state.behavior.music_reactions);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    reduce(&mut state, AppEvent::CloseSettings);
    assert!(!state.settings_open);
}

#[test]
fn test_shadow_settings_events_are_persisted() {
    let mut state = AppState::default();
    assert!(state.shadow.enabled);
    assert_eq!(state.shadow.intensity, 55);
    assert_eq!(state.shadow.radius, 18);
    assert_eq!(state.shadow.offset_x, 0);
    assert_eq!(state.shadow.offset_y, 6);

    let effects = reduce(&mut state, AppEvent::SetShadowEnabled(false));
    assert!(!state.shadow.enabled);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    let effects = reduce(&mut state, AppEvent::SetShadowIntensity(80));
    assert_eq!(state.shadow.intensity, 80);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    let effects = reduce(&mut state, AppEvent::SetShadowRadius(34));
    assert_eq!(state.shadow.radius, 34);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    let effects = reduce(&mut state, AppEvent::SetShadowOffsetX(-8));
    assert_eq!(state.shadow.offset_x, -8);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    let effects = reduce(&mut state, AppEvent::SetShadowOffsetY(12));
    assert_eq!(state.shadow.offset_y, 12);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    reduce(&mut state, AppEvent::SetShadowIntensity(999));
    assert_eq!(state.shadow.intensity, 100);
    reduce(&mut state, AppEvent::SetShadowRadius(999));
    assert_eq!(state.shadow.radius, 48);
    reduce(&mut state, AppEvent::SetShadowOffsetX(-99));
    assert_eq!(state.shadow.offset_x, -24);
    reduce(&mut state, AppEvent::SetShadowOffsetY(99));
    assert_eq!(state.shadow.offset_y, 24);
}

#[test]
fn test_reset_settings_restores_defaults_and_persists() {
    let mut state = AppState::default();
    state.blob_character = 6;
    state.blob_palette = 11;
    state.blob_expression = 14;
    state.blob_expression_custom = true;
    state.avatar_random = true;
    state.behavior.cursor_gaze = false;
    state.behavior.music_reactions = false;
    state.shadow.enabled = false;
    state.shadow.intensity = 80;
    state.shadow.radius = 34;
    state.shadow.offset_x = -8;
    state.shadow.offset_y = 12;
    state.music_visual_mode = island_core::models::MusicVisualMode::PetPrimary;

    let effects = reduce(&mut state, AppEvent::ResetSettings);

    assert_eq!(state.blob_character, 1);
    assert_eq!(state.blob_palette, 0);
    assert_eq!(state.blob_expression, 0);
    assert!(!state.blob_expression_custom);
    assert!(!state.avatar_random);
    assert!(state.behavior.cursor_gaze);
    assert!(state.behavior.music_reactions);
    assert!(state.shadow.enabled);
    assert_eq!(state.shadow.intensity, 55);
    assert_eq!(state.shadow.radius, 18);
    assert_eq!(state.shadow.offset_x, 0);
    assert_eq!(state.shadow.offset_y, 6);
    assert_eq!(
        state.music_visual_mode,
        island_core::models::MusicVisualMode::ArtworkPrimary
    );
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);
}

#[test]
fn test_click_stunts_can_be_disabled() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::SetClickStunts(false));
    let generation = state.mochi_generation;

    let effects = reduce(&mut state, AppEvent::PetClicked);

    assert!(effects.is_empty());
    assert_eq!(state.mochi_generation, generation);
    assert!(!state.pet_is_happy);
}

#[test]
fn test_prayer_alert_sets_and_clears_alert() {
    let mut state = AppState::default();
    let alert = island_core::prayer::PrayerAlert {
        prayer: island_core::prayer::Prayer::Fajr,
        kind: island_core::prayer::PrayerAlertKind::Adhan,
        at: chrono::Utc::now(),
        title: "Waktu Subuh".to_string(),
        time_label: "04:09".to_string(),
        city_label: "Jakarta".to_string(),
    };

    let effects = reduce(&mut state, AppEvent::PrayerAlert(alert));

    assert_eq!(state.alert_kind, island_core::models::AlertKind::Prayer);
    assert_eq!(state.active_module, ActiveModule::Alert);
    assert_eq!(state.prayer_alert.as_ref().map(|a| a.title.as_str()), Some("Waktu Subuh"));
    assert_eq!(
        effects,
        vec![island_core::Effect::ScheduleTimer {
            id: "alert_timeout",
            duration_ms: 9000,
            generation: 1,
        }]
    );

    reduce(&mut state, AppEvent::AlertTimeout(1));
    assert_eq!(state.alert_kind, island_core::models::AlertKind::None);
    assert!(state.prayer_alert.is_none());
}

#[test]
fn test_prayer_settings_events_are_persisted() {
    let mut state = AppState::default();

    let effects = reduce(&mut state, AppEvent::SetPrayerEnabled(false));
    assert!(!state.prayer.enabled);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    reduce(&mut state, AppEvent::SetPrayerCity(8));
    assert_eq!(state.prayer.city, 8);

    reduce(&mut state, AppEvent::SetPrayerReminderMinutes(15));
    assert_eq!(state.prayer.reminder_minutes, 15);
}
