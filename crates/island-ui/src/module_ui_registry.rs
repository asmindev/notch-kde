use island_core::{IslandView, ModuleId, ModuleRegistry};
use std::fmt;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ModuleUiSlot {
    System = 0,
    Media = 1,
    Alert = 2,
    Prayer = 3,
}

impl ModuleUiSlot {
    pub const FALLBACK: Self = Self::System;

    pub const fn index(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleUiSlots {
    pub compact: Option<ModuleUiSlot>,
    pub wide: Option<ModuleUiSlot>,
    pub expand: Option<ModuleUiSlot>,
}

impl ModuleUiSlots {
    const fn all(slot: ModuleUiSlot) -> Self {
        Self {
            compact: Some(slot),
            wide: Some(slot),
            expand: Some(slot),
        }
    }

    fn for_tier(self, tier: IslandView) -> Option<ModuleUiSlot> {
        match tier {
            IslandView::Compact => self.compact,
            IslandView::Wide => self.wide,
            IslandView::Expand => self.expand,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleUiBinding {
    pub id: ModuleId,
    pub slots: ModuleUiSlots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingModuleUiSlot {
    pub id: ModuleId,
    pub tier: IslandView,
}

impl fmt::Display for MissingModuleUiSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "module '{}' has no UI slot for {} tier",
            self.id,
            tier_name(self.tier)
        )
    }
}

const DEFAULT_BINDINGS: [ModuleUiBinding; 4] = [
    ModuleUiBinding {
        id: ModuleId::SYSTEM,
        slots: ModuleUiSlots::all(ModuleUiSlot::System),
    },
    ModuleUiBinding {
        id: ModuleId::MEDIA,
        slots: ModuleUiSlots::all(ModuleUiSlot::Media),
    },
    ModuleUiBinding {
        id: ModuleId::ALERT,
        slots: ModuleUiSlots {
            compact: Some(ModuleUiSlot::Alert),
            wide: Some(ModuleUiSlot::Alert),
            expand: None,
        },
    },
    ModuleUiBinding {
        id: ModuleId::PRAYER,
        slots: ModuleUiSlots {
            compact: None,
            wide: Some(ModuleUiSlot::Prayer),
            expand: Some(ModuleUiSlot::Prayer),
        },
    },
];

#[derive(Debug, Clone, Copy)]
pub struct ModuleUiRegistry {
    bindings: &'static [ModuleUiBinding],
}

impl Default for ModuleUiRegistry {
    fn default() -> Self {
        Self {
            bindings: &DEFAULT_BINDINGS,
        }
    }
}

impl ModuleUiRegistry {
    pub fn slot_for(&self, id: ModuleId, tier: IslandView) -> ModuleUiSlot {
        let Some(binding) = self.bindings.iter().find(|binding| binding.id == id) else {
            warn!(module = %id, tier = tier_name(tier), "module has no UI binding; falling back to system");
            return ModuleUiSlot::FALLBACK;
        };

        match binding.slots.for_tier(tier) {
            Some(slot) => slot,
            None => {
                warn!(module = %id, tier = tier_name(tier), "module has no UI slot; falling back to system");
                ModuleUiSlot::FALLBACK
            }
        }
    }

    pub fn validate(&self, modules: &ModuleRegistry) -> Vec<MissingModuleUiSlot> {
        let mut issues = Vec::new();
        for module in modules.iter() {
            let descriptor = module.descriptor();
            for tier in [IslandView::Compact, IslandView::Wide, IslandView::Expand] {
                if !descriptor.tiers.supports(tier) {
                    continue;
                }
                let has_slot = self
                    .bindings
                    .iter()
                    .find(|binding| binding.id == descriptor.id)
                    .and_then(|binding| binding.slots.for_tier(tier))
                    .is_some();
                if !has_slot {
                    issues.push(MissingModuleUiSlot {
                        id: descriptor.id,
                        tier,
                    });
                }
            }
        }
        issues
    }
}

fn tier_name(tier: IslandView) -> &'static str {
    match tier {
        IslandView::Compact => "compact",
        IslandView::Wide => "wide",
        IslandView::Expand => "expand",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use island_core::ModuleRegistry;

    #[test]
    fn built_in_modules_map_to_their_slots() {
        let registry = ModuleUiRegistry::default();

        assert_eq!(
            registry.slot_for(ModuleId::SYSTEM, IslandView::Compact),
            ModuleUiSlot::System
        );
        assert_eq!(
            registry.slot_for(ModuleId::MEDIA, IslandView::Expand),
            ModuleUiSlot::Media
        );
        assert_eq!(
            registry.slot_for(ModuleId::ALERT, IslandView::Wide),
            ModuleUiSlot::Alert
        );
        assert_eq!(
            registry.slot_for(ModuleId::PRAYER, IslandView::Expand),
            ModuleUiSlot::Prayer
        );
    }

    #[test]
    fn missing_tier_and_unknown_module_fall_back_to_system() {
        let registry = ModuleUiRegistry::default();

        assert_eq!(
            registry.slot_for(ModuleId::PRAYER, IslandView::Compact),
            ModuleUiSlot::System
        );
        assert_eq!(
            registry.slot_for(ModuleId::ALERT, IslandView::Expand),
            ModuleUiSlot::System
        );
        assert_eq!(
            registry.slot_for(ModuleId::new("extension"), IslandView::Wide),
            ModuleUiSlot::System
        );
    }

    #[test]
    fn validation_reports_supported_tiers_without_slots() {
        let issues = ModuleUiRegistry::default().validate(&ModuleRegistry::with_defaults());

        assert_eq!(
            issues,
            [MissingModuleUiSlot {
                id: ModuleId::ALERT,
                tier: IslandView::Expand,
            }]
        );
    }
}
