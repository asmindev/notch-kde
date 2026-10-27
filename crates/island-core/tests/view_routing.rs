use island_core::{reduce, AppEvent, AppState, IslandMode};

#[test]
fn test_view_routing_hover_wide() {
    let mut state = AppState::default();
    state.mode = IslandMode::Compact;

    // Hover true expands Compact to Wide
    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.mode, IslandMode::Wide);

    // Hover false retracts Wide back to Compact
    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert_eq!(state.mode, IslandMode::Compact);
}

#[test]
fn test_view_routing_click_expand_and_collapse() {
    let mut state = AppState::default();
    state.mode = IslandMode::Compact;

    // Click expands Compact to ExpandView
    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.mode, IslandMode::Expand);

    // Collapse click retracts Expand back to Compact
    reduce(&mut state, AppEvent::CollapseClicked);
    assert_eq!(state.mode, IslandMode::Compact);
}

#[test]
fn test_collapse_suppresses_immediate_hover_bounce() {
    let mut state = AppState::default();
    state.mode = IslandMode::Compact;

    // 1. Expand to ExpandView
    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.mode, IslandMode::Expand);

    // 2. Cursor was hovered over collapse button
    state.engine.is_hovered = true;
    reduce(&mut state, AppEvent::CollapseClicked);
    assert_eq!(state.mode, IslandMode::Compact);
    assert!(state.engine.hover_suppressed);

    // 3. Immediate hover event while cursor is still over pill does NOT bounce to Wide
    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.mode, IslandMode::Compact);

    // 4. Cursor exits the notch area -> re-arms hover
    reduce(&mut state, AppEvent::UserHoverChanged(false));
    assert_eq!(state.mode, IslandMode::Compact);
    assert!(!state.engine.hover_suppressed);

    // 5. Subsequent deliberate hover now successfully blooms to Wide
    reduce(&mut state, AppEvent::UserHoverChanged(true));
    assert_eq!(state.mode, IslandMode::Wide);
}

#[test]
fn test_expand_click_collapses_directly_to_compact() {
    let mut state = AppState::default();
    state.mode = IslandMode::Compact;

    // Expand via click
    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.mode, IslandMode::Expand);

    // Clicking again on expand view collapses directly to Compact
    reduce(&mut state, AppEvent::IslandClicked);
    assert_eq!(state.mode, IslandMode::Compact);
    assert!(state.engine.hover_suppressed);
}

#[test]
fn test_view_routing_manual_switch() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::ViewSwitchRequested(IslandMode::Wide));
    assert_eq!(state.mode, IslandMode::Wide);

    reduce(&mut state, AppEvent::ViewSwitchRequested(IslandMode::Expand));
    assert_eq!(state.mode, IslandMode::Expand);
}
