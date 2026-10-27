use island_core::TransitionPolicy;

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
