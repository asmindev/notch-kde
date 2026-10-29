use island_core::{AppEvent, AppState, IslandMode, reduce};

#[test]
fn test_view_routing_hover_wide() {
    let mut state = AppState::default();
    reduce(
        &mut state,
        AppEvent::ViewSwitchRequested(IslandMode::Compact),
    );

    // Hover true expands Compact to Wide
    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.view.mode(), IslandMode::Wide);

    // Hover false retracts Wide back to Compact
    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert_eq!(state.view.mode(), IslandMode::Compact);
}

#[test]
fn test_view_routing_click_expand_and_collapse() {
    let mut state = AppState::default();
    reduce(
        &mut state,
        AppEvent::ViewSwitchRequested(IslandMode::Compact),
    );

    // Click expands Compact to ExpandView
    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.view.mode(), IslandMode::Expand);

    // Collapse click follows the Wide intermediate stage before Compact
    let effects = reduce(&mut state, AppEvent::CollapseClicked);
    assert_eq!(state.view.mode(), IslandMode::Wide);
    let generation = state.runtime.overlap_dismiss_generation;
    assert_eq!(
        effects,
        vec![island_core::Effect::ScheduleTimer {
            id: "collapse_compact",
            duration_ms: 250,
            generation,
        }]
    );
    reduce(&mut state, AppEvent::CollapseCompactTimeout(generation));
    assert_eq!(state.view.mode(), IslandMode::Compact);
}

#[test]
fn test_collapse_stages_through_wide_before_compact() {
    let mut state = AppState::default();
    reduce(
        &mut state,
        AppEvent::ViewSwitchRequested(IslandMode::Compact),
    );

    // 1. Expand to ExpandView
    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.view.mode(), IslandMode::Expand);

    // 2. Collapse starts with the same Wide intermediate stage as overlap dismissal
    state.view.engine.is_hovered = true;
    reduce(&mut state, AppEvent::CollapseClicked);
    assert_eq!(state.view.mode(), IslandMode::Wide);
    assert!(state.view.engine.hover_suppressed);

    // 3. The scheduled transition completes at Compact
    let generation = state.runtime.overlap_dismiss_generation;
    reduce(&mut state, AppEvent::CollapseCompactTimeout(generation));
    assert_eq!(state.view.mode(), IslandMode::Compact);

    // 4. Cursor exits the notch area -> re-arms hover
    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert_eq!(state.view.mode(), IslandMode::Compact);
    assert!(!state.view.engine.hover_suppressed);

    // 5. Subsequent deliberate hover now successfully blooms to Wide
    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.view.mode(), IslandMode::Wide);
}

#[test]
fn test_expand_click_collapses_directly_to_compact() {
    let mut state = AppState::default();
    reduce(
        &mut state,
        AppEvent::ViewSwitchRequested(IslandMode::Compact),
    );

    // Expand via click
    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.view.mode(), IslandMode::Expand);

    // Clicking again on expand view follows the same staged collapse
    let effects = reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.view.mode(), IslandMode::Wide);
    let generation = state.runtime.overlap_dismiss_generation;
    assert_eq!(
        effects,
        vec![island_core::Effect::ScheduleTimer {
            id: "collapse_compact",
            duration_ms: 250,
            generation,
        }]
    );
    reduce(&mut state, AppEvent::CollapseCompactTimeout(generation));
    assert_eq!(state.view.mode(), IslandMode::Compact);
    assert!(state.view.engine.hover_suppressed);
}

#[test]
fn test_view_routing_manual_switch() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::ViewSwitchRequested(IslandMode::Wide));
    assert_eq!(state.view.mode(), IslandMode::Wide);

    reduce(
        &mut state,
        AppEvent::ViewSwitchRequested(IslandMode::Expand),
    );
    assert_eq!(state.view.mode(), IslandMode::Expand);
}
