use crate::models::ActiveModule;
use crate::view_engine::{ModuleTiers, Priority};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleDescriptor {
    pub id: ModuleId,
    pub name: &'static str,
    pub tiers: ModuleTiers,
    pub default_priority: Priority,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            ActiveModule::try_from(id),
            Err(UnknownModuleId(id))
        );
    }
}
