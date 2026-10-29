use crate::content_resolver::ResolvedContent;
use crate::models::IslandView;
use crate::module::ModuleId;
pub use crate::module::{IslandModule, ModuleTiers, Priority, SlotContext};
use crate::module_registry::{DuplicateModuleId, ModuleRegistry};
use crate::transition_policy::TransitionPolicy;
use std::sync::Arc;

/// ViewEngine manages the 3-tier view state machine and dodge sequencing
/// through clean composition of modules and priority resolution.
#[derive(Debug, Clone)]
pub struct ViewEngine {
    pub current_view: IslandView,
    pub previous_view: IslandView,
    pub(crate) active_module: ModuleId,
    pub is_visible: bool,
    pub is_overlapping: bool,
    pub is_hovered: bool,
    pub hover_suppressed: bool,
    pub overlap_reveal_hold: bool,
    pub overlap_dismiss_active: bool,
    dodge_enabled: bool,
    modules: ModuleRegistry,
}

impl PartialEq for ViewEngine {
    fn eq(&self, other: &Self) -> bool {
        self.current_view == other.current_view
            && self.previous_view == other.previous_view
            && self.active_module == other.active_module
            && self.is_visible == other.is_visible
            && self.is_overlapping == other.is_overlapping
            && self.is_hovered == other.is_hovered
            && self.hover_suppressed == other.hover_suppressed
            && self.overlap_reveal_hold == other.overlap_reveal_hold
            && self.overlap_dismiss_active == other.overlap_dismiss_active
            && self.dodge_enabled == other.dodge_enabled
    }
}

impl Default for ViewEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewEngine {
    pub fn new() -> Self {
        Self::with_registry(ModuleRegistry::with_defaults())
    }

    pub fn with_registry(modules: ModuleRegistry) -> Self {
        Self {
            current_view: IslandView::Wide,
            previous_view: IslandView::Wide,
            active_module: ModuleId::SYSTEM,
            is_visible: true,
            is_overlapping: false,
            is_hovered: false,
            hover_suppressed: false,
            overlap_reveal_hold: false,
            overlap_dismiss_active: false,
            dodge_enabled: true,
            modules,
        }
    }

    pub fn registry(&self) -> &ModuleRegistry {
        &self.modules
    }

    pub fn register_module(
        &mut self,
        module: Arc<dyn IslandModule>,
    ) -> Result<(), DuplicateModuleId> {
        self.modules.register(module)
    }

    pub fn active_module(&self) -> ModuleId {
        self.active_module
    }

    pub fn set_dodge_enabled(&mut self, enabled: bool) {
        self.dodge_enabled = enabled;
        self.recompute_visibility();
    }

    pub fn dodge_enabled(&self) -> bool {
        self.dodge_enabled
    }

    pub(crate) fn apply_resolved(&mut self, resolved: ResolvedContent) {
        self.active_module = resolved.module;
    }

    /// Sets the active view tier
    pub fn set_view(&mut self, target: IslandView) {
        if self.current_view != target {
            self.previous_view = self.current_view;
            self.current_view = target;
        }
        self.recompute_visibility();
    }

    fn recompute_visibility(&mut self) {
        self.is_visible = TransitionPolicy::evaluate_for_view(
            self.is_overlapping,
            self.is_hovered,
            self.overlap_reveal_hold,
            self.overlap_dismiss_active,
            self.current_view,
            self.dodge_enabled,
        );
    }

    pub fn reveal_overlap_compact(&mut self) {
        self.overlap_reveal_hold = true;
        self.overlap_dismiss_active = false;
        self.hover_suppressed = true;
        self.recompute_visibility();
    }

    pub fn finish_overlap_reveal(&mut self) {
        if self.overlap_reveal_hold && self.is_hovered && self.current_view == IslandView::Compact {
            self.overlap_reveal_hold = false;
            self.hover_suppressed = false;
            self.set_view(IslandView::Wide);
        }
    }

    pub fn begin_overlap_dismiss(&mut self) {
        self.overlap_dismiss_active = true;
        self.overlap_reveal_hold = false;
        self.hover_suppressed = true;
        self.set_view(IslandView::Wide);
    }

    pub fn begin_hover_overlap_dismiss(&mut self) {
        self.overlap_dismiss_active = true;
        self.overlap_reveal_hold = false;
        self.hover_suppressed = false;
        self.is_hovered = false;
        self.set_view(IslandView::Compact);
    }

    pub fn advance_overlap_dismiss(&mut self) {
        if self.overlap_dismiss_active {
            self.set_view(IslandView::Compact);
        }
    }

    pub fn finish_overlap_dismiss(&mut self) {
        if self.overlap_dismiss_active {
            self.overlap_dismiss_active = false;
            self.recompute_visibility();
        }
    }

    pub fn cancel_overlap_sequences(&mut self) {
        self.overlap_reveal_hold = false;
        self.overlap_dismiss_active = false;
        self.hover_suppressed = false;
        self.recompute_visibility();
    }

    /// Handles mouse hover interaction:
    /// - Hidden Compact reveals in place and blooms to Wide after the reveal timer.
    /// - Hover = true: expands visible Compact -> Wide unless suppressed.
    /// - Hover = false: collapses Wide -> Compact and clears reveal suppression.
    pub fn handle_hover(&mut self, hovered: bool) {
        if self.overlap_dismiss_active {
            self.is_hovered = hovered;
            if hovered {
                self.overlap_dismiss_active = false;
                self.hover_suppressed = false;
                if self.is_overlapping && self.current_view == IslandView::Compact {
                    self.set_view(IslandView::Wide);
                }
            }
            self.recompute_visibility();
            return;
        }

        let was_hidden = !self.is_visible;

        if !hovered {
            self.hover_suppressed = false;
            self.overlap_reveal_hold = false;
        }

        self.is_hovered = hovered;

        if hovered && was_hidden && self.is_overlapping && self.current_view == IslandView::Compact
        {
            self.overlap_reveal_hold = true;
            self.hover_suppressed = true;
            self.recompute_visibility();
            return;
        }

        if hovered {
            if !self.hover_suppressed && self.current_view == IslandView::Compact {
                self.set_view(IslandView::Wide);
            }
        } else if self.current_view == IslandView::Wide {
            self.set_view(IslandView::Compact);
        }

        self.recompute_visibility();
    }

    /// Handles click interaction:
    /// - Compact / Wide -> ExpandView!
    /// - ExpandView -> collapses back directly to CompactView
    pub fn handle_click(&mut self) {
        self.overlap_reveal_hold = false;
        self.hover_suppressed = false;

        match self.current_view {
            IslandView::Compact | IslandView::Wide => {
                self.set_view(IslandView::Expand);
            }
            IslandView::Expand => {
                self.is_hovered = false;
                self.set_view(IslandView::Compact);
                self.hover_suppressed = true;
            }
        }
    }

    /// Handles explicit collapse button [ v ]:
    /// Retracts from ExpandView directly back to CompactView and suppresses hover
    pub fn handle_collapse(&mut self) {
        self.overlap_reveal_hold = false;
        self.is_hovered = false;
        self.set_view(IslandView::Compact);
        self.hover_suppressed = true;
    }

    /// Handles window overlap dodge events without changing the active view.
    /// Visibility is recomputed from overlap, hover, reveal, and dismissal state.
    pub fn handle_overlap(&mut self, overlapping: bool) {
        self.is_overlapping = overlapping;

        if !overlapping {
            if self.overlap_dismiss_active {
                self.overlap_dismiss_active = false;
                self.hover_suppressed = false;
            }
            if self.overlap_reveal_hold {
                if self.is_hovered {
                    self.finish_overlap_reveal();
                } else {
                    self.overlap_reveal_hold = false;
                    self.hover_suppressed = false;
                }
            }
        }

        self.recompute_visibility();
    }

    /// Returns tier capabilities for any given module
    pub fn module_tiers(&self, module: ModuleId) -> ModuleTiers {
        self.modules
            .iter()
            .find(|m| m.descriptor().id == module)
            .map(|m| m.descriptor().tiers)
            .unwrap_or(ModuleTiers::ALL)
    }
}
