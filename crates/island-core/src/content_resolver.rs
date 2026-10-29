use crate::models::IslandView;
use crate::module::{ModuleContext, ModuleId, Priority};
use crate::module_registry::ModuleRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentReason {
    Priority,
    UserPanel,
    UserSelector,
    Fallback,
    UnsupportedTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedContent {
    pub module: ModuleId,
    pub priority: Priority,
    pub reason: ContentReason,
}

#[derive(Debug, Clone, Copy)]
pub struct ContentResolver<'a> {
    registry: &'a ModuleRegistry,
}

impl<'a> ContentResolver<'a> {
    pub fn new(registry: &'a ModuleRegistry) -> Self {
        Self { registry }
    }

    pub fn resolve(&self, ctx: &ModuleContext<'_>, tier: IslandView) -> ResolvedContent {
        let mut best: Option<(ModuleId, Priority, ContentReason)> = None;

        for module in self.registry.iter() {
            let descriptor = module.descriptor();
            if !descriptor.tiers.supports(tier) {
                continue;
            }

            let (priority, reason) = effective_priority(module.as_ref(), ctx, descriptor.id);
            let wins = best
                .as_ref()
                .is_none_or(|(_, best_priority, _)| priority > *best_priority);
            if wins {
                best = Some((descriptor.id, priority, reason));
            }
        }

        let unsupported_override = requested_override_is_unsupported(self.registry, ctx, tier);

        if let Some((module, priority, mut reason)) = best
            && priority > Priority::INACTIVE
        {
            if module == ModuleId::SYSTEM {
                if unsupported_override {
                    reason = ContentReason::UnsupportedTier;
                } else if reason == ContentReason::Priority && priority <= Priority::LOW {
                    reason = ContentReason::Fallback;
                }
            }
            return ResolvedContent {
                module,
                priority,
                reason,
            };
        }

        ResolvedContent {
            module: ModuleId::SYSTEM,
            priority: Priority::INACTIVE,
            reason: if unsupported_override {
                ContentReason::UnsupportedTier
            } else {
                ContentReason::Fallback
            },
        }
    }
}

pub fn resolve(
    registry: &ModuleRegistry,
    ctx: &ModuleContext<'_>,
    tier: IslandView,
) -> ResolvedContent {
    ContentResolver::new(registry).resolve(ctx, tier)
}

fn effective_priority(
    module: &dyn crate::module::IslandModule,
    ctx: &ModuleContext<'_>,
    id: ModuleId,
) -> (Priority, ContentReason) {
    let mut priority = module.priority(ctx);
    let mut reason = ContentReason::Priority;

    if ctx.prayer_panel_open && id == ModuleId::PRAYER {
        priority = priority.max(Priority::PANEL);
        reason = ContentReason::UserPanel;
    }
    if ctx.source_selector_open && id == ModuleId::MEDIA {
        priority = priority.max(Priority::NORMAL);
        reason = ContentReason::UserSelector;
    }

    (priority, reason)
}

fn requested_override_is_unsupported(
    registry: &ModuleRegistry,
    ctx: &ModuleContext<'_>,
    tier: IslandView,
) -> bool {
    (ctx.prayer_panel_open && !supports_tier(registry, ModuleId::PRAYER, tier))
        || (ctx.source_selector_open && !supports_tier(registry, ModuleId::MEDIA, tier))
}

fn supports_tier(registry: &ModuleRegistry, id: ModuleId, tier: IslandView) -> bool {
    registry
        .get(id)
        .is_some_and(|module| module.descriptor().tiers.supports(tier))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AlertKind, BatteryState, MediaTrack, TimeInfo};

    fn context<'a>(
        time: &'a TimeInfo,
        battery: &'a BatteryState,
        media: &'a MediaTrack,
        tier: IslandView,
        alert_kind: AlertKind,
        prayer_panel_open: bool,
        source_selector_open: bool,
    ) -> ModuleContext<'a> {
        ModuleContext {
            time,
            battery,
            media,
            view: tier,
            is_muted: false,
            is_dnd: false,
            alert_kind,
            prayer_panel_open,
            source_selector_open,
        }
    }

    #[test]
    fn playing_media_wins_by_priority_in_all_tiers() {
        let registry = ModuleRegistry::with_defaults();
        let time = TimeInfo::default();
        let battery = BatteryState::default();
        let media = MediaTrack {
            title: "Song".to_string(),
            is_playing: true,
            ..MediaTrack::default()
        };

        for tier in [IslandView::Compact, IslandView::Wide, IslandView::Expand] {
            let ctx = context(&time, &battery, &media, tier, AlertKind::None, false, false);
            let resolved = resolve(&registry, &ctx, tier);
            assert_eq!(resolved.module, ModuleId::MEDIA);
            assert_eq!(resolved.reason, ContentReason::Priority);
        }
    }

    #[test]
    fn source_selector_forces_paused_media_with_reason() {
        let registry = ModuleRegistry::with_defaults();
        let time = TimeInfo::default();
        let battery = BatteryState::default();
        let media = MediaTrack {
            title: "Paused".to_string(),
            ..MediaTrack::default()
        };
        let ctx = context(
            &time,
            &battery,
            &media,
            IslandView::Wide,
            AlertKind::None,
            false,
            true,
        );

        let resolved = resolve(&registry, &ctx, IslandView::Wide);
        assert_eq!(resolved.module, ModuleId::MEDIA);
        assert_eq!(resolved.reason, ContentReason::UserSelector);
        assert_eq!(resolved.priority, Priority::NORMAL);
    }

    #[test]
    fn prayer_panel_respects_tier_and_reports_reason() {
        let registry = ModuleRegistry::with_defaults();
        let time = TimeInfo::default();
        let battery = BatteryState::default();
        let media = MediaTrack::default();

        let wide = context(
            &time,
            &battery,
            &media,
            IslandView::Wide,
            AlertKind::None,
            true,
            false,
        );
        let resolved_wide = resolve(&registry, &wide, IslandView::Wide);
        assert_eq!(resolved_wide.module, ModuleId::PRAYER);
        assert_eq!(resolved_wide.reason, ContentReason::UserPanel);

        let compact = context(
            &time,
            &battery,
            &media,
            IslandView::Compact,
            AlertKind::None,
            true,
            false,
        );
        let resolved_compact = resolve(&registry, &compact, IslandView::Compact);
        assert_eq!(resolved_compact.module, ModuleId::SYSTEM);
        assert_eq!(resolved_compact.reason, ContentReason::UnsupportedTier);
    }

    #[test]
    fn alert_outranks_panel_and_selector_overrides() {
        let registry = ModuleRegistry::with_defaults();
        let time = TimeInfo::default();
        let battery = BatteryState::default();
        let media = MediaTrack {
            title: "Paused".to_string(),
            ..MediaTrack::default()
        };
        let ctx = context(
            &time,
            &battery,
            &media,
            IslandView::Wide,
            AlertKind::Charging,
            true,
            true,
        );

        let resolved = resolve(&registry, &ctx, IslandView::Wide);
        assert_eq!(resolved.module, ModuleId::ALERT);
        assert_eq!(resolved.reason, ContentReason::Priority);
    }
}
