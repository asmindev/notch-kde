use crate::models::IslandView;

pub struct TransitionPolicy;

impl TransitionPolicy {
    /// Determines whether the island should be visible based on overlap and hover.
    pub fn evaluate_visibility(is_overlapping: bool, is_hovered: bool) -> bool {
        Self::evaluate_for_view(
            is_overlapping,
            is_hovered,
            false,
            false,
            IslandView::Compact,
            true,
        )
    }

    /// Determines visibility for the current view and overlap transition.
    ///
    /// Core stays deterministic: callers provide the deployment flag instead of
    /// relying on process environment state.
    pub fn evaluate_for_view(
        is_overlapping: bool,
        is_hovered: bool,
        reveal_hold: bool,
        dismiss_active: bool,
        view: IslandView,
        dodge_enabled: bool,
    ) -> bool {
        !dodge_enabled
            || dismiss_active
            || reveal_hold
            || !is_overlapping
            || is_hovered
            || view != IslandView::Compact
    }
}
