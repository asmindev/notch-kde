use crate::models::IslandView;

pub struct TransitionPolicy;

impl TransitionPolicy {
    /// Determines whether the island should be visible based on overlap and hover
    pub fn evaluate_visibility(is_overlapping: bool, is_hovered: bool) -> bool {
        Self::evaluate_for_view(
            is_overlapping,
            is_hovered,
            false,
            false,
            IslandView::Compact,
        )
    }

    /// Determines visibility for the current view and overlap transition.
    pub fn evaluate_for_view(
        is_overlapping: bool,
        is_hovered: bool,
        reveal_hold: bool,
        dismiss_active: bool,
        view: IslandView,
    ) -> bool {
        if std::env::var("ISLAND_DODGE").map(|v| v == "0").unwrap_or(false) {
            return true;
        }

        dismiss_active
            || reveal_hold
            || !is_overlapping
            || is_hovered
            || view != IslandView::Compact
    }

    /// Resolves target view on user hover interaction
    pub fn resolve_view_on_hover(current: IslandView, is_hovered: bool) -> IslandView {
        match current {
            IslandView::Compact => {
                if is_hovered {
                    IslandView::Wide
                } else {
                    IslandView::Compact
                }
            }
            IslandView::Wide => {
                if is_hovered {
                    IslandView::Wide
                } else {
                    IslandView::Compact
                }
            }
            IslandView::Expand => {
                if is_hovered {
                    IslandView::Expand
                } else {
                    IslandView::Compact
                }
            }
        }
    }

    /// Resolves target view on click
    pub fn resolve_view_on_click(current: IslandView) -> IslandView {
        match current {
            IslandView::Compact | IslandView::Wide => IslandView::Expand,
            IslandView::Expand => IslandView::Wide,
        }
    }
}
