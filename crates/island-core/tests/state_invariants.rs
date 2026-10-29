use island_core::models::IslandView;
use island_core::transition_policy::TransitionPolicy;
use island_core::{AppEvent, AppState, MediaTrack, ModuleId, reduce};

fn assert_invariants(state: &AppState) {
    let engine = &state.view.engine;
    let active = state.view.active_module();

    assert_eq!(active, engine.active_module());
    assert_eq!(state.view.mode(), engine.current_view);
    assert!(
        engine.module_tiers(active).supports(state.view.mode()),
        "module {active} does not support tier {:?}",
        state.view.mode()
    );

    let expected_visible = TransitionPolicy::evaluate_for_view(
        engine.is_overlapping,
        engine.is_hovered,
        engine.overlap_reveal_hold,
        engine.overlap_dismiss_active,
        engine.current_view,
    );
    assert_eq!(state.view.is_visible(), expected_visible);
    assert_eq!(state.view.is_overlapping(), engine.is_overlapping);
    assert_eq!(state.view.is_hovered(), engine.is_hovered);
}

fn playing_media() -> MediaTrack {
    MediaTrack {
        title: "Song".to_string(),
        is_playing: true,
        ..MediaTrack::default()
    }
}

#[test]
fn active_module_stays_supported_after_view_events() {
    let mut state = AppState::default();
    assert_invariants(&state);

    let events = [
        AppEvent::WelcomeTimeout,
        AppEvent::MediaUpdated(playing_media()),
        AppEvent::TogglePrayerPanel,
        AppEvent::ViewSwitchRequested(IslandView::Wide),
        AppEvent::ViewSwitchRequested(IslandView::Compact),
        AppEvent::ViewSwitchRequested(IslandView::Expand),
        AppEvent::ToggleSourceSelector,
        AppEvent::ToggleSourceSelector,
        AppEvent::WindowOverlapChanged(true),
        AppEvent::UserHoverChanged(false),
        AppEvent::UserHoverChanged(true),
        AppEvent::WindowOverlapChanged(false),
        AppEvent::IslandClicked,
        AppEvent::CollapseClicked,
        AppEvent::MediaUpdated(MediaTrack::default()),
        AppEvent::ModuleSwitchRequested(island_core::ActiveModule::Prayer),
    ];

    for event in events {
        reduce(&mut state, event);
        assert_invariants(&state);
    }
}

#[test]
fn prayer_module_is_dropped_when_new_tier_does_not_support_it() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::TogglePrayerPanel);
    assert_eq!(state.view.active_module(), ModuleId::PRAYER);
    assert_invariants(&state);

    reduce(
        &mut state,
        AppEvent::ViewSwitchRequested(IslandView::Compact),
    );
    assert_eq!(state.view.mode(), IslandView::Compact);
    assert_eq!(state.view.active_module(), ModuleId::SYSTEM);
    assert_invariants(&state);
}

#[test]
fn visibility_tracks_overlap_and_hover_engine_state() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::WelcomeTimeout);
    assert_invariants(&state);

    reduce(&mut state, AppEvent::WindowOverlapChanged(true));
    assert!(!state.view.is_visible());
    assert_invariants(&state);

    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert!(state.view.is_visible());
    assert_invariants(&state);

    let reveal_generation = state.runtime.overlap_reveal_generation;
    reduce(
        &mut state,
        AppEvent::OverlapRevealTimeout(reveal_generation),
    );
    assert_eq!(state.view.mode(), IslandView::Wide);
    assert_invariants(&state);

    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert_invariants(&state);
    let dismiss_generation = state.runtime.overlap_dismiss_generation;

    reduce(
        &mut state,
        AppEvent::OverlapDismissCompactTimeout(dismiss_generation),
    );
    assert_invariants(&state);

    reduce(
        &mut state,
        AppEvent::OverlapDismissHideTimeout(dismiss_generation),
    );
    assert!(!state.view.is_visible());
    assert_invariants(&state);
}
