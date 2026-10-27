use crate::models::{ActiveModule, AlertKind, BatteryState, IslandView, MediaTrack, TimeInfo};
use crate::transition_policy::TransitionPolicy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Priority representation for slot arbitration.
/// Higher value = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Priority(pub u32);

impl Priority {
    pub const INACTIVE: Self = Self(0);
    pub const LOW: Self = Self(10);       // System / Idle default
    pub const NORMAL: Self = Self(50);    // Paused media / background status
    pub const HIGH: Self = Self(100);     // Active media playback / running timer
    pub const URGENT: Self = Self(200);   // Important alerts / incoming notifications
    pub const CRITICAL: Self = Self(300); // Critical battery (<10%) / emergency alerts
}

/// Lightweight snapshot of state passed into modules to evaluate priority.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotContext<'a> {
    pub time: &'a TimeInfo,
    pub battery: &'a BatteryState,
    pub media: &'a MediaTrack,
    pub view: IslandView,
    pub is_muted: bool,
    pub is_dnd: bool,
    pub alert_kind: AlertKind,
}

/// Defines which view tiers a module can render into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleTiers {
    pub compact: bool,
    pub wide: bool,
    pub expand: bool,
}

impl ModuleTiers {
    pub const ALL: Self = Self { compact: true, wide: true, expand: true };
    pub const COMPACT_WIDE: Self = Self { compact: true, wide: true, expand: false };
    pub const WIDE_ONLY: Self = Self { compact: false, wide: true, expand: false };

    pub fn supports(&self, view: IslandView) -> bool {
        match view {
            IslandView::Compact => self.compact,
            IslandView::Wide => self.wide,
            IslandView::Expand => self.expand,
        }
    }
}

/// Trait defining modular content behavior for Dynamic Island slots.
pub trait IslandModule: std::fmt::Debug + Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn module_type(&self) -> ActiveModule;
    fn tiers(&self) -> ModuleTiers;
    fn priority(&self, ctx: &SlotContext) -> Priority;
}

/// Concrete System Module (Clock, Battery, Controls - Supports all 3 tiers)
#[derive(Debug, Default)]
pub struct SystemModule;

impl IslandModule for SystemModule {
    fn id(&self) -> &'static str { "system" }
    fn name(&self) -> &'static str { "System Overview" }
    fn module_type(&self) -> ActiveModule { ActiveModule::System }
    fn tiers(&self) -> ModuleTiers { ModuleTiers::ALL }
    fn priority(&self, ctx: &SlotContext) -> Priority {
        if ctx.battery.percentage <= 10 && !ctx.battery.is_charging {
            Priority::CRITICAL
        } else {
            Priority::LOW
        }
    }
}

/// Concrete Media Module (Supports all 3 tiers: Compact, Wide, Expand)
#[derive(Debug, Default)]
pub struct MediaModule;

impl IslandModule for MediaModule {
    fn id(&self) -> &'static str { "media" }
    fn name(&self) -> &'static str { "Now Playing" }
    fn module_type(&self) -> ActiveModule { ActiveModule::Media }
    fn tiers(&self) -> ModuleTiers { ModuleTiers::ALL }
    fn priority(&self, ctx: &SlotContext) -> Priority {
        if ctx.media.is_playing {
            Priority::HIGH
        } else if ctx.view == IslandView::Compact && !ctx.media.title.is_empty() {
            Priority::NORMAL
        } else {
            Priority::INACTIVE
        }
    }
}

/// Concrete Alert Module (Temporary notification - Supports Wide tier)
#[derive(Debug, Default)]
pub struct AlertModule;

impl IslandModule for AlertModule {
    fn id(&self) -> &'static str { "alert" }
    fn name(&self) -> &'static str { "Notification" }
    fn module_type(&self) -> ActiveModule { ActiveModule::Alert }
    fn tiers(&self) -> ModuleTiers { ModuleTiers::ALL }
    fn priority(&self, ctx: &SlotContext) -> Priority {
        if ctx.alert_kind != AlertKind::None {
            Priority::URGENT
        } else {
            Priority::INACTIVE
        }
    }
}

/// ViewEngine manages the 3-tier view state machine and dodge sequencing
/// through clean composition of modules and priority resolution.
#[derive(Debug, Clone)]
pub struct ViewEngine {
    pub current_view: IslandView,
    pub previous_view: IslandView,
    pub active_module: ActiveModule,
    pub is_visible: bool,
    pub is_overlapping: bool,
    pub is_hovered: bool,
    pub hover_suppressed: bool,
    pub overlap_reveal_hold: bool,
    pub overlap_dismiss_active: bool,
    modules: Vec<Arc<dyn IslandModule>>,
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
    }
}

impl Default for ViewEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            current_view: IslandView::Wide,
            previous_view: IslandView::Wide,
            active_module: ActiveModule::System,
            is_visible: true,
            is_overlapping: false,
            is_hovered: false,
            hover_suppressed: false,
            overlap_reveal_hold: false,
            overlap_dismiss_active: false,
            modules: Vec::new(),
        };

        // Register default modules
        engine.register_module(Arc::new(SystemModule));
        engine.register_module(Arc::new(MediaModule));
        engine.register_module(Arc::new(AlertModule));

        engine
    }

    pub fn register_module(&mut self, module: Arc<dyn IslandModule>) {
        self.modules.push(module);
    }

    /// Resolves the active module based on highest priority among modules
    /// supporting the current view tier.
    pub fn resolve_active_module(&mut self, ctx: &SlotContext) {
        let best_module = self.modules
            .iter()
            .filter(|m| m.tiers().supports(self.current_view))
            .max_by_key(|m| m.priority(ctx));

        if let Some(module) = best_module {
            // Only switch if the best module has an active priority
            if module.priority(ctx) > Priority::INACTIVE {
                self.active_module = module.module_type();
            } else {
                self.active_module = ActiveModule::System;
            }
        }
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
        );
    }

    pub fn reveal_overlap_compact(&mut self) {
        self.overlap_reveal_hold = true;
        self.overlap_dismiss_active = false;
        self.hover_suppressed = true;
        self.recompute_visibility();
    }

    pub fn finish_overlap_reveal(&mut self) {
        if self.overlap_reveal_hold
            && self.is_hovered
            && self.current_view == IslandView::Compact
        {
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

    /// Compatibility method for media updates
    pub fn handle_media_update(&mut self, track: &MediaTrack) {
        let has_media = !track.title.is_empty();

        if track.is_playing || (self.current_view == IslandView::Compact && has_media) {
            self.active_module = ActiveModule::Media;
        } else {
            self.active_module = ActiveModule::System;
        }
    }

    /// Returns tier capabilities for any given module
    pub fn module_tiers(&self, module: ActiveModule) -> ModuleTiers {
        self.modules
            .iter()
            .find(|m| m.module_type() == module)
            .map(|m| m.tiers())
            .unwrap_or(ModuleTiers::ALL)
    }
}
