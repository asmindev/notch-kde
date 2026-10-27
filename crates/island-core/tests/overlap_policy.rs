use island_core::{IslandView, TransitionPolicy};

#[test]
fn test_transition_policy_visibility() {
    // No overlap, not hovered -> visible
    assert!(TransitionPolicy::evaluate_visibility(false, false));

    // Overlap, not hovered -> hidden
    assert!(!TransitionPolicy::evaluate_visibility(true, false));

    // Overlap, but hovered -> visible (user inspection takes precedence)
    assert!(TransitionPolicy::evaluate_visibility(true, true));

    // No overlap, hovered -> visible
    assert!(TransitionPolicy::evaluate_visibility(false, true));
}

#[test]
fn test_transition_policy_view_aware_visibility() {
    assert!(!TransitionPolicy::evaluate_for_view(
        true,
        false,
        false,
        false,
        IslandView::Compact,
    ));
    assert!(TransitionPolicy::evaluate_for_view(
        true,
        false,
        true,
        false,
        IslandView::Compact,
    ));
    assert!(TransitionPolicy::evaluate_for_view(
        true,
        false,
        false,
        false,
        IslandView::Wide,
    ));
    assert!(TransitionPolicy::evaluate_for_view(
        true,
        false,
        false,
        true,
        IslandView::Compact,
    ));
}
