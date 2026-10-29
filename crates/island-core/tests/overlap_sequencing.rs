use island_core::{AppEvent, AppState, Effect, IslandView, reduce};

fn compact_state() -> AppState {
    let mut state = AppState::default();
    reduce(
        &mut state,
        AppEvent::ViewSwitchRequested(IslandView::Compact),
    );
    assert_eq!(state.view.mode, IslandView::Compact);
    state
}

fn hidden_by_overlap() -> AppState {
    let mut state = compact_state();
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));
    assert!(state.view.is_overlapping);
    assert!(!state.view.is_visible);
    state
}

#[test]
fn hover_reveals_compact_then_wide_after_timeout() {
    let mut state = hidden_by_overlap();

    let effects = reduce(&mut state, AppEvent::UserHoverChanged(true));
    let generation = state.runtime.overlap_reveal_generation;
    assert!(state.view.is_visible);
    assert_eq!(state.view.mode, IslandView::Compact);
    assert_eq!(
        effects,
        vec![Effect::ScheduleTimer {
            id: "overlap_reveal",
            duration_ms: 300,
            generation,
        }]
    );

    reduce(&mut state, AppEvent::OverlapRevealTimeout(generation - 1));
    assert_eq!(state.view.mode, IslandView::Compact);

    reduce(&mut state, AppEvent::OverlapRevealTimeout(generation));
    assert_eq!(state.view.mode, IslandView::Wide);

    let effects = reduce(&mut state, AppEvent::UserHoverChanged(false));
    let hide_generation = state.runtime.overlap_dismiss_generation;
    assert_eq!(state.view.mode, IslandView::Compact);
    assert!(state.view.is_visible);
    assert!(state.view.engine.overlap_dismiss_active);
    assert_eq!(
        effects,
        vec![Effect::ScheduleTimer {
            id: "overlap_dismiss_hide",
            duration_ms: 250,
            generation: hide_generation,
        }]
    );

    reduce(
        &mut state,
        AppEvent::OverlapDismissHideTimeout(hide_generation - 1),
    );
    assert!(state.view.is_visible);
    assert_eq!(state.view.mode, IslandView::Compact);

    reduce(
        &mut state,
        AppEvent::OverlapDismissHideTimeout(hide_generation),
    );
    assert!(!state.view.is_visible);
    assert_eq!(state.view.mode, IslandView::Compact);
}

#[test]
fn leaving_before_reveal_timeout_cancels_reveal() {
    let mut state = hidden_by_overlap();

    reduce(&mut state, AppEvent::UserHoverChanged(true));
    let generation = state.runtime.overlap_reveal_generation;

    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert!(!state.view.is_visible);
    assert_eq!(state.view.mode, IslandView::Compact);

    reduce(&mut state, AppEvent::OverlapRevealTimeout(generation));
    assert!(!state.view.is_visible);
    assert_eq!(state.view.mode, IslandView::Compact);
}

#[test]
fn clicking_hidden_notch_reveals_compact_only() {
    let mut state = hidden_by_overlap();

    let effects = reduce(&mut state, AppEvent::IslandClicked);
    assert!(state.view.is_visible);
    assert_eq!(state.view.mode, IslandView::Compact);
    assert!(effects.is_empty());

    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.view.mode, IslandView::Expand);
}

#[test]
fn expand_stays_open_while_overlapping() {
    let mut state = compact_state();
    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.view.mode, IslandView::Expand);

    reduce(&mut state, AppEvent::WindowOverlapChanged(true));
    assert_eq!(state.view.mode, IslandView::Expand);
    assert!(state.view.is_visible);
}

#[test]
fn clicking_expand_notch_dismisses_in_stages() {
    let mut state = compact_state();
    reduce(&mut state, AppEvent::IslandClicked);
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));

    let effects = reduce(&mut state, AppEvent::IslandClicked);
    let generation = state.runtime.overlap_dismiss_generation;
    assert_eq!(state.view.mode, IslandView::Wide);
    assert!(state.view.is_visible);
    assert!(state.view.engine.overlap_dismiss_active);
    assert_eq!(
        effects,
        vec![Effect::ScheduleTimer {
            id: "overlap_dismiss_compact",
            duration_ms: 250,
            generation,
        }]
    );

    reduce(
        &mut state,
        AppEvent::OverlapDismissCompactTimeout(generation - 1),
    );
    assert_eq!(state.view.mode, IslandView::Wide);

    let effects = reduce(
        &mut state,
        AppEvent::OverlapDismissCompactTimeout(generation),
    );
    assert_eq!(state.view.mode, IslandView::Compact);
    assert!(state.view.is_visible);
    assert_eq!(
        effects,
        vec![Effect::ScheduleTimer {
            id: "overlap_dismiss_hide",
            duration_ms: 250,
            generation,
        }]
    );

    reduce(&mut state, AppEvent::OverlapDismissHideTimeout(generation));
    assert!(!state.view.is_visible);
    assert_eq!(state.view.mode, IslandView::Compact);
    assert!(!state.view.engine.overlap_dismiss_active);
}

#[test]
fn clicking_outside_expand_dismisses_in_stages() {
    let mut state = compact_state();
    reduce(&mut state, AppEvent::IslandClicked);
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));

    let effects = reduce(&mut state, AppEvent::CollapseClicked);
    let generation = state.runtime.overlap_dismiss_generation;
    assert_eq!(state.view.mode, IslandView::Wide);
    assert!(state.view.is_visible);
    assert_eq!(
        effects,
        vec![Effect::ScheduleTimer {
            id: "overlap_dismiss_compact",
            duration_ms: 250,
            generation,
        }]
    );
}

#[test]
fn hover_reentry_cancels_dismissal() {
    let mut state = compact_state();
    reduce(&mut state, AppEvent::IslandClicked);
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));

    reduce(&mut state, AppEvent::CollapseClicked);
    let generation = state.runtime.overlap_dismiss_generation;

    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert!(!state.view.engine.overlap_dismiss_active);
    assert!(state.view.is_visible);
    assert_eq!(state.view.mode, IslandView::Wide);

    reduce(
        &mut state,
        AppEvent::OverlapDismissCompactTimeout(generation),
    );
    assert_eq!(state.view.mode, IslandView::Wide);
}

#[test]
fn cursor_leaving_during_dismissal_does_not_cancel_it() {
    let mut state = compact_state();
    reduce(&mut state, AppEvent::IslandClicked);
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));

    reduce(&mut state, AppEvent::CollapseClicked);
    let generation = state.runtime.overlap_dismiss_generation;

    reduce(
        &mut state,
        AppEvent::OverlapDismissCompactTimeout(generation),
    );
    assert_eq!(state.view.mode, IslandView::Compact);
    assert!(state.view.is_visible);

    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert!(state.view.engine.overlap_dismiss_active);

    reduce(&mut state, AppEvent::OverlapDismissHideTimeout(generation));
    assert!(!state.view.is_visible);
    assert_eq!(state.view.mode, IslandView::Compact);
}

#[test]
fn overlap_clearing_cancels_dismissal() {
    let mut state = compact_state();
    reduce(&mut state, AppEvent::IslandClicked);
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));

    reduce(&mut state, AppEvent::CollapseClicked);
    let generation = state.runtime.overlap_dismiss_generation;
    assert_eq!(state.view.mode, IslandView::Wide);

    reduce(&mut state, AppEvent::WindowOverlapChanged(false));
    assert!(!state.view.engine.overlap_dismiss_active);
    assert!(state.view.is_visible);

    reduce(
        &mut state,
        AppEvent::OverlapDismissCompactTimeout(generation),
    );
    assert_eq!(state.view.mode, IslandView::Wide);
}

#[test]
fn overlap_clearing_during_reveal_blooms_immediately_when_hovered() {
    let mut state = hidden_by_overlap();

    reduce(&mut state, AppEvent::UserHoverChanged(true));
    let generation = state.runtime.overlap_reveal_generation;
    assert_eq!(state.view.mode, IslandView::Compact);

    reduce(&mut state, AppEvent::WindowOverlapChanged(false));
    assert_eq!(state.view.mode, IslandView::Wide);
    assert!(state.view.is_visible);

    reduce(&mut state, AppEvent::OverlapRevealTimeout(generation));
    assert_eq!(state.view.mode, IslandView::Wide);
}

#[test]
fn clicking_during_dismissal_cancels_the_sequence() {
    let mut state = compact_state();
    reduce(&mut state, AppEvent::IslandClicked);
    reduce(&mut state, AppEvent::WindowOverlapChanged(true));

    reduce(&mut state, AppEvent::CollapseClicked);
    let generation = state.runtime.overlap_dismiss_generation;
    assert_eq!(state.view.mode, IslandView::Wide);

    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.view.mode, IslandView::Expand);
    assert!(!state.view.engine.overlap_dismiss_active);

    reduce(
        &mut state,
        AppEvent::OverlapDismissCompactTimeout(generation),
    );
    assert_eq!(state.view.mode, IslandView::Expand);
}

#[test]
fn hover_reentry_at_compact_cancels_hide_and_restores_wide() {
    let mut state = hidden_by_overlap();

    reduce(&mut state, AppEvent::UserHoverChanged(true));
    let reveal_generation = state.runtime.overlap_reveal_generation;
    reduce(
        &mut state,
        AppEvent::OverlapRevealTimeout(reveal_generation),
    );
    assert_eq!(state.view.mode, IslandView::Wide);

    reduce(&mut state, AppEvent::UserHoverChanged(false));
    let hide_generation = state.runtime.overlap_dismiss_generation;
    assert_eq!(state.view.mode, IslandView::Compact);
    assert!(state.view.is_visible);

    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.view.mode, IslandView::Wide);
    assert!(state.view.is_visible);
    assert!(!state.view.engine.overlap_dismiss_active);

    reduce(
        &mut state,
        AppEvent::OverlapDismissHideTimeout(hide_generation),
    );
    assert_eq!(state.view.mode, IslandView::Wide);
    assert!(state.view.is_visible);
}
