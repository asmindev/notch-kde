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
fn test_view_routing_manual_switch() {
    let mut state = AppState::default();
    reduce(&mut state, AppEvent::ViewSwitchRequested(IslandMode::Wide));
    assert_eq!(state.mode, IslandMode::Wide);

    reduce(&mut state, AppEvent::ViewSwitchRequested(IslandMode::Expand));
    assert_eq!(state.mode, IslandMode::Expand);
}
