use crate::models::IslandView;

pub struct TransitionPolicy;

impl TransitionPolicy {
    /// Determines whether the island should be visible based on overlap and hover
    pub fn evaluate_visibility(is_overlapping: bool, is_hovered: bool) -> bool {
        if std::env::var("ISLAND_DODGE").map(|v| v == "0").unwrap_or(false) {
            return true;
        }
        if is_hovered {
            // User hovering over the notch always keeps it visible
            true
        } else {
            // If windows overlap the notch area and user isn't hovering, hide it
            !is_overlapping
        }
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
