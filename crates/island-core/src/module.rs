use crate::models::{ActiveModule, AlertKind, BatteryState, IslandView, MediaTrack, TimeInfo};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub &'static str);

impl ModuleId {
    pub const SYSTEM: Self = Self("system");
    pub const MEDIA: Self = Self("media");
    pub const ALERT: Self = Self("alert");
    pub const PRAYER: Self = Self("prayer");

    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }

    pub fn active_module(self) -> Option<ActiveModule> {
        ActiveModule::try_from(self).ok()
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl PartialEq<ActiveModule> for ModuleId {
    fn eq(&self, other: &ActiveModule) -> bool {
        self.active_module() == Some(*other)
    }
}

impl PartialEq<ModuleId> for ActiveModule {
    fn eq(&self, other: &ModuleId) -> bool {
        other == self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownModuleId(pub ModuleId);

impl fmt::Display for UnknownModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown module id: {}", self.0)
    }
}

impl std::error::Error for UnknownModuleId {}

/// Priority representation for slot arbitration.
/// Higher value = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Priority(pub u32);

impl Priority {
    pub const INACTIVE: Self = Self(0);
    pub const LOW: Self = Self(10);
    pub const NORMAL: Self = Self(50);
    pub const HIGH: Self = Self(100);
    pub const PANEL: Self = Self(150);
    pub const URGENT: Self = Self(200);
    pub const CRITICAL: Self = Self(300);
}

/// Lightweight read-only snapshot passed into modules and the content resolver.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleContext<'a> {
    pub time: &'a TimeInfo,
    pub battery: &'a BatteryState,
    pub media: &'a MediaTrack,
    pub view: IslandView,
    pub is_muted: bool,
    pub is_dnd: bool,
    pub alert_kind: AlertKind,
    pub prayer_panel_open: bool,
    pub source_selector_open: bool,
}

/// Backwards-compatible name used by existing modules.
pub type SlotContext<'a> = ModuleContext<'a>;

/// Defines which view tiers a module can render into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleTiers {
    pub compact: bool,
    pub wide: bool,
    pub expand: bool,
}

impl ModuleTiers {
    pub const ALL: Self = Self {
        compact: true,
        wide: true,
        expand: true,
    };
    pub const COMPACT_WIDE: Self = Self {
        compact: true,
        wide: true,
        expand: false,
    };
    pub const WIDE_ONLY: Self = Self {
        compact: false,
        wide: true,
        expand: false,
    };
    pub const WIDE_EXPAND: Self = Self {
        compact: false,
        wide: true,
        expand: true,
    };

    pub fn supports(&self, view: IslandView) -> bool {
        match view {
            IslandView::Compact => self.compact,
            IslandView::Wide => self.wide,
            IslandView::Expand => self.expand,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleDescriptor {
    pub id: ModuleId,
    pub name: &'static str,
    pub tiers: ModuleTiers,
    pub default_priority: Priority,
}

/// Trait defining modular content behavior for Dynamic Island slots.
pub trait IslandModule: fmt::Debug + Send + Sync {
    fn descriptor(&self) -> ModuleDescriptor;
    fn priority(&self, ctx: &ModuleContext<'_>) -> Priority;
}

/// Concrete System Module (Clock, Battery, Controls - Supports all 3 tiers)
#[derive(Debug, Default)]
pub struct SystemModule;

impl IslandModule for SystemModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId::SYSTEM,
            name: "System Overview",
            tiers: ModuleTiers::ALL,
            default_priority: Priority::LOW,
        }
    }

    fn priority(&self, ctx: &ModuleContext<'_>) -> Priority {
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
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId::MEDIA,
            name: "Now Playing",
            tiers: ModuleTiers::ALL,
            default_priority: Priority::NORMAL,
        }
    }

    fn priority(&self, ctx: &ModuleContext<'_>) -> Priority {
        if ctx.media.is_playing {
            Priority::HIGH
        } else if ctx.view == IslandView::Compact && !ctx.media.title.is_empty() {
            Priority::NORMAL
        } else {
            Priority::INACTIVE
        }
    }
}

/// Concrete Prayer Module (User-selected schedule panel - Supports Wide/Expand)
#[derive(Debug, Default)]
pub struct PrayerModule;

impl IslandModule for PrayerModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId::PRAYER,
            name: "Jadwal Sholat",
            tiers: ModuleTiers::WIDE_EXPAND,
            default_priority: Priority::PANEL,
        }
    }

    fn priority(&self, ctx: &ModuleContext<'_>) -> Priority {
        if ctx.prayer_panel_open {
            Priority::PANEL
        } else {
            Priority::INACTIVE
        }
    }
}

/// Concrete Alert Module (Temporary notification - Supports all tiers)
#[derive(Debug, Default)]
pub struct AlertModule;

impl IslandModule for AlertModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId::ALERT,
            name: "Notification",
            tiers: ModuleTiers::ALL,
            default_priority: Priority::URGENT,
        }
    }

    fn priority(&self, ctx: &ModuleContext<'_>) -> Priority {
        if ctx.alert_kind != AlertKind::None {
            Priority::URGENT
        } else {
            Priority::INACTIVE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context<'a>(
        time: &'a TimeInfo,
        battery: &'a BatteryState,
        media: &'a MediaTrack,
        alert_kind: AlertKind,
        prayer_panel_open: bool,
    ) -> ModuleContext<'a> {
        ModuleContext {
            time,
            battery,
            media,
            view: IslandView::Wide,
            is_muted: false,
            is_dnd: false,
            alert_kind,
            prayer_panel_open,
            source_selector_open: false,
        }
    }

    #[test]
    fn active_module_round_trips_through_module_id() {
        for module in [
            ActiveModule::System,
            ActiveModule::Media,
            ActiveModule::Alert,
            ActiveModule::Prayer,
        ] {
            let id = ModuleId::from(module);
            assert_eq!(id.active_module(), Some(module));
            assert_eq!(id, module);
        }
    }

    #[test]
    fn unknown_module_id_is_rejected() {
        let id = ModuleId::new("extension");
        assert_eq!(id.as_str(), "extension");
        assert_eq!(id.active_module(), None);
        assert_eq!(ActiveModule::try_from(id), Err(UnknownModuleId(id)));
    }

    #[test]
    fn descriptors_are_stable() {
        assert_eq!(
            SystemModule.descriptor(),
            ModuleDescriptor {
                id: ModuleId::SYSTEM,
                name: "System Overview",
                tiers: ModuleTiers::ALL,
                default_priority: Priority::LOW,
            }
        );
        assert_eq!(
            MediaModule.descriptor(),
            ModuleDescriptor {
                id: ModuleId::MEDIA,
                name: "Now Playing",
                tiers: ModuleTiers::ALL,
                default_priority: Priority::NORMAL,
            }
        );
        assert_eq!(
            AlertModule.descriptor(),
            ModuleDescriptor {
                id: ModuleId::ALERT,
                name: "Notification",
                tiers: ModuleTiers::ALL,
                default_priority: Priority::URGENT,
            }
        );
        assert_eq!(
            PrayerModule.descriptor(),
            ModuleDescriptor {
                id: ModuleId::PRAYER,
                name: "Jadwal Sholat",
                tiers: ModuleTiers::WIDE_EXPAND,
                default_priority: Priority::PANEL,
            }
        );
    }

    #[test]
    fn priorities_match_existing_behavior() {
        let time = TimeInfo::default();
        let media = MediaTrack::default();
        let mut battery = BatteryState::default();
        let ctx = context(&time, &battery, &media, AlertKind::None, false);

        assert_eq!(SystemModule.priority(&ctx), Priority::LOW);
        assert_eq!(MediaModule.priority(&ctx), Priority::INACTIVE);
        assert_eq!(AlertModule.priority(&ctx), Priority::INACTIVE);
        assert_eq!(PrayerModule.priority(&ctx), Priority::INACTIVE);

        battery.percentage = 8;
        battery.is_charging = false;
        let critical = context(&time, &battery, &media, AlertKind::None, false);
        assert_eq!(SystemModule.priority(&critical), Priority::CRITICAL);

        let playing_media = MediaTrack {
            is_playing: true,
            ..MediaTrack::default()
        };
        let playing = context(&time, &battery, &playing_media, AlertKind::None, false);
        assert_eq!(MediaModule.priority(&playing), Priority::HIGH);

        let alert = context(&time, &battery, &media, AlertKind::Charging, false);
        assert_eq!(AlertModule.priority(&alert), Priority::URGENT);

        let prayer = context(&time, &battery, &media, AlertKind::None, true);
        assert_eq!(PrayerModule.priority(&prayer), Priority::PANEL);
    }
}
