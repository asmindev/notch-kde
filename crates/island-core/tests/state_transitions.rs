use island_core::{ActiveModule, AppEvent, AppState, IslandMode, MediaTrack, reduce};

fn fire_welcome(state: &mut AppState) {
    let generation = state.presentation.welcome_generation;
    reduce(
        state,
        AppEvent::TimerFired {
            id: island_core::TimerId::Welcome,
            generation,
        },
    );
}

#[test]
fn test_initial_state_wide_welcome() {
    let state = AppState::default();
    assert_eq!(state.view.mode(), IslandMode::Wide);
    assert!(state.presentation.is_welcome);
    assert!(state.view.is_visible());
    assert!(!state.view.is_overlapping());
    assert_eq!(state.view.active_module(), ActiveModule::System);
}

#[test]
fn test_welcome_timeout_to_compact() {
    let mut state = AppState::default();
    assert!(state.presentation.is_welcome);
    assert_eq!(state.view.mode(), IslandMode::Wide);

    fire_welcome(&mut state);
    assert_eq!(state.view.mode(), IslandMode::Compact);
    assert!(!state.presentation.is_welcome);
}

#[test]
fn test_screen_unlocked_triggers_welcome() {
    let mut state = AppState::default();
    // First settle to compact
    fire_welcome(&mut state);
    assert_eq!(state.view.mode(), IslandMode::Compact);
    assert!(!state.presentation.is_welcome);

    // Screen unlocks from lockscreen
    let effects = reduce(&mut state, AppEvent::ScreenUnlocked);
    assert_eq!(state.view.mode(), IslandMode::Wide);
    assert!(state.presentation.is_welcome);
    assert!(state.presentation.pet_is_happy);
    assert_eq!(
        effects,
        vec![
            island_core::events::Effect::ScheduleTimer {
                id: island_core::TimerId::PetHappy,
                duration_ms: 1500,
                generation: 1,
            },
            island_core::events::Effect::ScheduleTimer {
                id: island_core::TimerId::Welcome,
                duration_ms: 3500,
                generation: 2,
            },
        ]
    );

    // Timeout settles it back to compact
    fire_welcome(&mut state);
    assert_eq!(state.view.mode(), IslandMode::Compact);
    assert!(!state.presentation.is_welcome);
}

#[test]
fn test_overlap_hides_notch() {
    let mut state = AppState::default();
    reduce(
        &mut state,
        AppEvent::ViewSwitchRequested(IslandMode::Compact),
    );

    // Overlapping window arrives
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));
    assert_eq!(state.view.is_overlapping(), true);
    assert_eq!(state.view.is_visible(), false);

    // Overlapping window moves away
    reduce(&mut state, AppEvent::WindowOverlapChanged(false));
    assert_eq!(state.view.is_overlapping(), false);
    assert_eq!(state.view.is_visible(), true);
}

#[test]
fn test_hover_overrides_overlap() {
    let mut state = AppState::default();
    reduce(
        &mut state,
        AppEvent::ViewSwitchRequested(IslandMode::Compact),
    );

    // Window overlaps
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));
    assert_eq!(state.view.is_visible(), false);

    // User moves mouse over notch edge
    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.view.is_visible(), true);

    // User leaves mouse
    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert_eq!(state.view.is_visible(), false);
}

#[test]
fn test_media_playback_activates_media_module() {
    let mut state = AppState::default();
    reduce(
        &mut state,
        AppEvent::ViewSwitchRequested(IslandMode::Compact),
    );

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
    assert_eq!(state.view.active_module(), ActiveModule::Media);
}

#[test]
fn test_toggle_source_selector_and_select() {
    let mut state = AppState::default();
    assert!(!state.module.show_source_selector);

    // Toggle on
    reduce(&mut state, AppEvent::ToggleSourceSelector);
    assert!(state.module.show_source_selector);
    assert_eq!(state.view.active_module(), ActiveModule::Media);

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
    assert_eq!(state.domain.media_sources.len(), 2);

    // Select Spotify
    let effects = reduce(
        &mut state,
        AppEvent::SelectMediaSource("org.mpris.MediaPlayer2.spotify".to_string()),
    );
    assert_eq!(
        effects,
        vec![island_core::events::Effect::SelectMediaSource(
            "org.mpris.MediaPlayer2.spotify".to_string()
        )]
    );
    assert!(state.domain.media_sources[0].is_active);
    assert!(!state.domain.media_sources[1].is_active);

    // Toggle off
    reduce(&mut state, AppEvent::ToggleSourceSelector);
    assert!(!state.module.show_source_selector);
}

#[test]
fn test_pet_clicked_and_happy_timeout() {
    let mut state = AppState::default();
    assert!(!state.presentation.pet_is_happy);

    // Click pet
    let effects = reduce(&mut state, AppEvent::PetClicked);
    assert!(state.presentation.pet_is_happy);
    assert_eq!(state.presentation.pet_happy_generation, 1);
    assert!(
        effects.contains(&island_core::events::Effect::ScheduleTimer {
            id: island_core::TimerId::PetHappy,
            duration_ms: 1500,
            generation: 1,
        })
    );

    // Another click updates generation
    let effects2 = reduce(&mut state, AppEvent::PetClicked);
    assert!(state.presentation.pet_is_happy);
    assert_eq!(state.presentation.pet_happy_generation, 2);
    assert!(
        effects2.contains(&island_core::events::Effect::ScheduleTimer {
            id: island_core::TimerId::PetHappy,
            duration_ms: 1500,
            generation: 2,
        })
    );

    // Stale timeout generation 1 does not reset happy state
    reduce(
        &mut state,
        AppEvent::TimerFired {
            id: island_core::TimerId::PetHappy,
            generation: 1,
        },
    );
    assert!(state.presentation.pet_is_happy);

    // Matching timeout generation 2 resets happy state
    reduce(
        &mut state,
        AppEvent::TimerFired {
            id: island_core::TimerId::PetHappy,
            generation: 2,
        },
    );
    assert!(!state.presentation.pet_is_happy);
}

#[test]
fn test_pet_rapid_poking_triggers_dizzy() {
    let mut state = AppState::default();
    assert!(!state.presentation.pet_is_dizzy);
    assert_eq!(
        state.presentation.mochi_mood,
        island_core::state::MochiMoodState::Idle
    );

    // Poke 1, 2 times -> Annoyed (slaps < 3)
    reduce(&mut state, AppEvent::PetClicked);
    assert_eq!(
        state.presentation.mochi_mood,
        island_core::state::MochiMoodState::Annoyed
    );
    reduce(&mut state, AppEvent::PetClicked);
    assert_eq!(
        state.presentation.mochi_mood,
        island_core::state::MochiMoodState::Annoyed
    );
    assert!(!state.presentation.pet_is_dizzy);

    // Poke 3rd time -> triggers dizzy (Coucou 3-slap mechanic)!
    let effects = reduce(&mut state, AppEvent::PetClicked);
    assert!(state.presentation.pet_is_dizzy);
    assert_eq!(
        state.presentation.mochi_mood,
        island_core::state::MochiMoodState::Dizzy
    );
    assert!(!state.presentation.pet_is_happy);
    assert!(
        effects.contains(&island_core::events::Effect::ScheduleTimer {
            id: island_core::TimerId::MochiDizzy,
            duration_ms: 2200,
            generation: state.presentation.mochi_generation,
        })
    );

    // Dizzy timeout resets dizzy state
    let generation = state.presentation.mochi_generation;
    reduce(
        &mut state,
        AppEvent::TimerFired {
            id: island_core::TimerId::MochiDizzy,
            generation,
        },
    );
    assert!(!state.presentation.pet_is_dizzy);
    assert_eq!(
        state.presentation.mochi_mood,
        island_core::state::MochiMoodState::Idle
    );
    assert_eq!(state.presentation.mochi_slap_count, 0);
}

#[test]
fn test_pet_sleepy_on_low_battery() {
    let mut state = AppState::default();
    assert!(!state.presentation.pet_is_sleepy);
    assert_eq!(
        state.presentation.mochi_mood,
        island_core::state::MochiMoodState::Idle
    );

    // Battery drops to 15% and not charging -> sleepy
    reduce(
        &mut state,
        AppEvent::BatteryUpdated(island_core::models::BatteryState {
            percentage: 15,
            is_charging: false,
        }),
    );
    assert!(state.presentation.pet_is_sleepy);
    assert_eq!(
        state.presentation.mochi_mood,
        island_core::state::MochiMoodState::Sleeping
    );

    // Plug in charger -> wakes up!
    reduce(
        &mut state,
        AppEvent::BatteryUpdated(island_core::models::BatteryState {
            percentage: 15,
            is_charging: true,
        }),
    );
    assert!(!state.presentation.pet_is_sleepy);
    assert_eq!(
        state.presentation.mochi_mood,
        island_core::state::MochiMoodState::Idle
    );
}

#[test]
fn test_bloub_shape_and_palette_selection_and_cycle() {
    let mut state = AppState::default();
    assert_eq!(state.settings.blob_character, 1); // Default Galet
    assert_eq!(state.settings.blob_palette, 0); // Default notchde light

    // Cycle through all eight shapes: 1 -> 2 -> ... -> 7 -> 0 -> 1
    for expected in [2, 3, 4, 5, 6, 7, 0, 1] {
        reduce(&mut state, AppEvent::CycleBlobCharacter);
        assert_eq!(state.settings.blob_character, expected);
    }

    // Cycle through all fourteen palettes: 0 -> ... -> 13 -> 0
    for expected in 1..14 {
        reduce(&mut state, AppEvent::CycleBlobPalette);
        assert_eq!(state.settings.blob_palette, expected);
    }
    reduce(&mut state, AppEvent::CycleBlobPalette);
    assert_eq!(state.settings.blob_palette, 0);

    // Direct set wraps
    reduce(&mut state, AppEvent::SetBlobCharacter(9));
    assert_eq!(state.settings.blob_character, 1);
    reduce(&mut state, AppEvent::SetBlobPalette(15));
    assert_eq!(state.settings.blob_palette, 1);

    // Expression cycling and direct set wrap through all expressions and mark
    // the choice as an explicit custom override.
    assert_eq!(state.settings.blob_expression, 0);
    assert!(!state.settings.blob_expression_custom);
    for expected in 1..17 {
        reduce(&mut state, AppEvent::CycleBlobExpression);
        assert_eq!(state.settings.blob_expression, expected);
    }
    reduce(&mut state, AppEvent::CycleBlobExpression);
    assert_eq!(state.settings.blob_expression, 0);
    assert!(state.settings.blob_expression_custom);
    reduce(&mut state, AppEvent::SetBlobExpression(18));
    assert_eq!(state.settings.blob_expression, 1);
    assert!(state.settings.blob_expression_custom);

    // One-shot shuffle always changes the avatar, auto-random only when enabled.
    let before = (
        state.settings.blob_character,
        state.settings.blob_palette,
        state.settings.blob_expression,
    );
    reduce(&mut state, AppEvent::RandomizeAvatar);
    assert!(
        (
            state.settings.blob_character,
            state.settings.blob_palette,
            state.settings.blob_expression
        ) != before
    );
    let before_auto = (
        state.settings.blob_character,
        state.settings.blob_palette,
        state.settings.blob_expression,
    );
    reduce(&mut state, AppEvent::AvatarRandomTick);
    assert_eq!(
        (
            state.settings.blob_character,
            state.settings.blob_palette,
            state.settings.blob_expression
        ),
        before_auto
    );
    reduce(&mut state, AppEvent::ToggleAvatarRandom);
    assert!(state.settings.avatar_random);
    assert!(
        (
            state.settings.blob_character,
            state.settings.blob_palette,
            state.settings.blob_expression
        ) != before_auto
    );
}

#[test]
fn notification_event_bumps_generation() {
    let mut state = AppState::default();
    assert_eq!(state.presentation.notification_generation, 0);
    reduce(&mut state, AppEvent::NotificationReceived);
    reduce(&mut state, AppEvent::NotificationReceived);
    assert_eq!(state.presentation.notification_generation, 2);
}

#[test]
fn test_settings_events_open_tab_toggle_and_persist() {
    let mut state = AppState::default();

    reduce(&mut state, AppEvent::OpenSettings);
    assert!(state.presentation.settings_open);

    reduce(
        &mut state,
        AppEvent::SetSettingsTab(island_core::SettingsTab::System),
    );
    assert_eq!(
        state.presentation.settings_tab,
        island_core::SettingsTab::System
    );

    let effects = reduce(&mut state, AppEvent::SetMusicReactions(false));
    assert!(!state.settings.behavior.music_reactions);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    let effects = reduce(
        &mut state,
        AppEvent::SetPetRenderer(island_core::PetRenderer::TwoD),
    );
    assert_eq!(state.settings.pet_renderer, island_core::PetRenderer::TwoD);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    reduce(&mut state, AppEvent::CloseSettings);
    assert!(!state.presentation.settings_open);
}

#[test]
fn test_shadow_settings_events_are_persisted() {
    let mut state = AppState::default();
    assert!(state.settings.shadow.enabled);
    assert_eq!(state.settings.shadow.intensity, 55);
    assert_eq!(state.settings.shadow.radius, 18);
    assert_eq!(state.settings.shadow.offset_x, 0);
    assert_eq!(state.settings.shadow.offset_y, 6);

    let effects = reduce(&mut state, AppEvent::SetShadowEnabled(false));
    assert!(!state.settings.shadow.enabled);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    let effects = reduce(&mut state, AppEvent::SetShadowIntensity(80));
    assert_eq!(state.settings.shadow.intensity, 80);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    let effects = reduce(&mut state, AppEvent::SetShadowRadius(34));
    assert_eq!(state.settings.shadow.radius, 34);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    let effects = reduce(&mut state, AppEvent::SetShadowOffsetX(-8));
    assert_eq!(state.settings.shadow.offset_x, -8);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    let effects = reduce(&mut state, AppEvent::SetShadowOffsetY(12));
    assert_eq!(state.settings.shadow.offset_y, 12);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    reduce(&mut state, AppEvent::SetShadowIntensity(999));
    assert_eq!(state.settings.shadow.intensity, 100);
    reduce(&mut state, AppEvent::SetShadowRadius(999));
    assert_eq!(state.settings.shadow.radius, 48);
    reduce(&mut state, AppEvent::SetShadowOffsetX(-99));
    assert_eq!(state.settings.shadow.offset_x, -24);
    reduce(&mut state, AppEvent::SetShadowOffsetY(99));
    assert_eq!(state.settings.shadow.offset_y, 24);
}

#[test]
fn test_reset_settings_restores_defaults_and_persists() {
    let mut state = AppState::default();
    state.settings.blob_character = 6;
    state.settings.blob_palette = 11;
    state.settings.blob_expression = 14;
    state.settings.blob_expression_custom = true;
    state.settings.avatar_random = true;
    state.settings.behavior.cursor_gaze = false;
    state.settings.behavior.music_reactions = false;
    state.settings.shadow.enabled = false;
    state.settings.shadow.intensity = 80;
    state.settings.shadow.radius = 34;
    state.settings.shadow.offset_x = -8;
    state.settings.shadow.offset_y = 12;
    state.settings.music_visual_mode = island_core::models::MusicVisualMode::PetPrimary;
    state.settings.pet_renderer = island_core::PetRenderer::TwoD;

    let effects = reduce(&mut state, AppEvent::ResetSettings);

    assert_eq!(state.settings.blob_character, 1);
    assert_eq!(state.settings.blob_palette, 0);
    assert_eq!(state.settings.blob_expression, 0);
    assert!(!state.settings.blob_expression_custom);
    assert!(!state.settings.avatar_random);
    assert!(state.settings.behavior.cursor_gaze);
    assert!(state.settings.behavior.music_reactions);
    assert!(state.settings.shadow.enabled);
    assert_eq!(state.settings.shadow.intensity, 55);
    assert_eq!(state.settings.shadow.radius, 18);
    assert_eq!(state.settings.shadow.offset_x, 0);
    assert_eq!(state.settings.shadow.offset_y, 6);
    assert_eq!(
        state.settings.music_visual_mode,
        island_core::models::MusicVisualMode::ArtworkPrimary
    );
    assert_eq!(
        state.settings.pet_renderer,
        island_core::PetRenderer::ThreeD
    );
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);
}

#[test]
fn test_click_stunts_can_be_disabled() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::SetClickStunts(false));
    let generation = state.presentation.mochi_generation;

    let effects = reduce(&mut state, AppEvent::PetClicked);

    assert!(effects.is_empty());
    assert_eq!(state.presentation.mochi_generation, generation);
    assert!(!state.presentation.pet_is_happy);
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

    assert_eq!(
        state.domain.alert_kind,
        island_core::models::AlertKind::Prayer
    );
    assert_eq!(state.view.active_module(), ActiveModule::Alert);
    assert_eq!(
        state.domain.prayer_alert.as_ref().map(|a| a.title.as_str()),
        Some("Waktu Subuh")
    );
    assert_eq!(
        effects,
        vec![island_core::Effect::ScheduleTimer {
            id: island_core::TimerId::Alert,
            duration_ms: 9000,
            generation: 1,
        }]
    );

    reduce(
        &mut state,
        AppEvent::TimerFired {
            id: island_core::TimerId::Alert,
            generation: 1,
        },
    );
    assert_eq!(
        state.domain.alert_kind,
        island_core::models::AlertKind::None
    );
    assert!(state.domain.prayer_alert.is_none());
}

#[test]
fn test_prayer_settings_events_are_persisted() {
    let mut state = AppState::default();

    let effects = reduce(&mut state, AppEvent::SetPrayerEnabled(false));
    assert!(!state.settings.prayer.enabled);
    assert_eq!(effects, vec![island_core::Effect::PersistSettings]);

    reduce(&mut state, AppEvent::SetPrayerCity(8));
    assert_eq!(state.settings.prayer.city, 8);

    reduce(&mut state, AppEvent::SetPrayerReminderMinutes(15));
    assert_eq!(state.settings.prayer.reminder_minutes, 15);
}
