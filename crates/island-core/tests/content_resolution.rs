use std::sync::Arc;

use island_core::content_resolver::{ContentReason, resolve};
use island_core::models::{AlertKind, IslandView};
use island_core::{
    BatteryState, IslandModule, MediaTrack, ModuleContext, ModuleDescriptor, ModuleId,
    ModuleRegistry, ModuleTiers, Priority, TimeInfo,
};

#[derive(Debug)]
struct FixedModule {
    id: ModuleId,
    priority: Priority,
}

impl IslandModule for FixedModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: self.id,
            name: "Fixed",
            tiers: ModuleTiers::ALL,
            default_priority: self.priority,
        }
    }

    fn priority(&self, _ctx: &ModuleContext<'_>) -> Priority {
        self.priority
    }
}

#[derive(Default)]
struct Snapshot {
    time: TimeInfo,
    battery: BatteryState,
    media: MediaTrack,
}

impl Snapshot {
    fn context(
        &self,
        tier: IslandView,
        alert_kind: AlertKind,
        prayer_panel_open: bool,
        source_selector_open: bool,
    ) -> ModuleContext<'_> {
        ModuleContext {
            time: &self.time,
            battery: &self.battery,
            media: &self.media,
            view: tier,
            is_muted: false,
            is_dnd: false,
            alert_kind,
            prayer_panel_open,
            source_selector_open,
        }
    }
}

fn paused_media() -> MediaTrack {
    MediaTrack {
        title: "Paused".to_string(),
        artist: "Artist".to_string(),
        is_playing: false,
        ..MediaTrack::default()
    }
}

fn playing_media() -> MediaTrack {
    MediaTrack {
        is_playing: true,
        ..paused_media()
    }
}

#[test]
fn playing_media_resolves_in_every_tier() {
    let registry = ModuleRegistry::with_defaults();
    let mut snapshot = Snapshot {
        media: playing_media(),
        ..Snapshot::default()
    };
    snapshot.media.artist = "Artist".to_string();

    for tier in [IslandView::Compact, IslandView::Wide, IslandView::Expand] {
        let ctx = snapshot.context(tier, AlertKind::None, false, false);
        let resolved = resolve(&registry, &ctx, tier);
        assert_eq!(resolved.module, ModuleId::MEDIA, "tier: {tier:?}");
        assert_eq!(resolved.priority, Priority::HIGH, "tier: {tier:?}");
        assert_eq!(resolved.reason, ContentReason::Priority, "tier: {tier:?}");
    }
}

#[test]
fn paused_media_only_wins_in_compact() {
    let registry = ModuleRegistry::with_defaults();
    let snapshot = Snapshot {
        media: paused_media(),
        ..Snapshot::default()
    };

    let compact_ctx = snapshot.context(IslandView::Compact, AlertKind::None, false, false);
    let compact = resolve(&registry, &compact_ctx, IslandView::Compact);
    assert_eq!(compact.module, ModuleId::MEDIA);
    assert_eq!(compact.reason, ContentReason::Priority);

    for tier in [IslandView::Wide, IslandView::Expand] {
        let ctx = snapshot.context(tier, AlertKind::None, false, false);
        let resolved = resolve(&registry, &ctx, tier);
        assert_eq!(resolved.module, ModuleId::SYSTEM, "tier: {tier:?}");
        assert_eq!(resolved.reason, ContentReason::Fallback, "tier: {tier:?}");
    }
}

#[test]
fn idle_falls_back_to_system_with_reason() {
    let registry = ModuleRegistry::with_defaults();
    let snapshot = Snapshot::default();

    for tier in [IslandView::Compact, IslandView::Wide, IslandView::Expand] {
        let ctx = snapshot.context(tier, AlertKind::None, false, false);
        let resolved = resolve(&registry, &ctx, tier);
        assert_eq!(resolved.module, ModuleId::SYSTEM, "tier: {tier:?}");
        assert_eq!(resolved.reason, ContentReason::Fallback, "tier: {tier:?}");
    }
}

#[test]
fn alert_resolves_in_every_tier() {
    let registry = ModuleRegistry::with_defaults();
    let snapshot = Snapshot::default();

    for tier in [IslandView::Compact, IslandView::Wide, IslandView::Expand] {
        let ctx = snapshot.context(tier, AlertKind::Charging, false, false);
        let resolved = resolve(&registry, &ctx, tier);
        assert_eq!(resolved.module, ModuleId::ALERT, "tier: {tier:?}");
        assert_eq!(resolved.reason, ContentReason::Priority, "tier: {tier:?}");
    }
}

#[test]
fn alert_wins_over_panel_and_selector() {
    let registry = ModuleRegistry::with_defaults();
    let snapshot = Snapshot {
        media: playing_media(),
        ..Snapshot::default()
    };
    let ctx = snapshot.context(IslandView::Wide, AlertKind::Charging, true, true);

    let resolved = resolve(&registry, &ctx, IslandView::Wide);
    assert_eq!(resolved.module, ModuleId::ALERT);
    assert_eq!(resolved.reason, ContentReason::Priority);
}

#[test]
fn prayer_panel_wins_in_supported_tiers() {
    let registry = ModuleRegistry::with_defaults();
    let snapshot = Snapshot {
        media: playing_media(),
        ..Snapshot::default()
    };

    for tier in [IslandView::Wide, IslandView::Expand] {
        let ctx = snapshot.context(tier, AlertKind::None, true, false);
        let resolved = resolve(&registry, &ctx, tier);
        assert_eq!(resolved.module, ModuleId::PRAYER, "tier: {tier:?}");
        assert_eq!(resolved.reason, ContentReason::UserPanel, "tier: {tier:?}");
    }
}

#[test]
fn source_selector_forces_paused_media_in_every_tier() {
    let registry = ModuleRegistry::with_defaults();
    let snapshot = Snapshot {
        media: paused_media(),
        ..Snapshot::default()
    };

    for tier in [IslandView::Compact, IslandView::Wide, IslandView::Expand] {
        let ctx = snapshot.context(tier, AlertKind::None, false, true);
        let resolved = resolve(&registry, &ctx, tier);
        assert_eq!(resolved.module, ModuleId::MEDIA, "tier: {tier:?}");
        assert_eq!(
            resolved.reason,
            ContentReason::UserSelector,
            "tier: {tier:?}"
        );
    }
}

#[test]
fn unsupported_prayer_panel_reports_unsupported_tier() {
    let registry = ModuleRegistry::with_defaults();
    let snapshot = Snapshot::default();
    let ctx = snapshot.context(IslandView::Compact, AlertKind::None, true, false);

    let resolved = resolve(&registry, &ctx, IslandView::Compact);
    assert_eq!(resolved.module, ModuleId::SYSTEM);
    assert_eq!(resolved.reason, ContentReason::UnsupportedTier);
}

#[test]
fn equal_priority_uses_registration_order() {
    let mut registry = ModuleRegistry::new();
    registry
        .register(Arc::new(FixedModule {
            id: ModuleId::new("first"),
            priority: Priority::HIGH,
        }))
        .expect("register first");
    registry
        .register(Arc::new(FixedModule {
            id: ModuleId::new("second"),
            priority: Priority::HIGH,
        }))
        .expect("register second");

    let snapshot = Snapshot::default();
    let ctx = snapshot.context(IslandView::Wide, AlertKind::None, false, false);
    let resolved = resolve(&registry, &ctx, IslandView::Wide);

    assert_eq!(resolved.module, ModuleId::new("first"));
    assert_eq!(resolved.reason, ContentReason::Priority);
}
