use crate::module::ModuleId;
use crate::view_engine::{
    AlertModule, IslandModule, MediaModule, PrayerModule, SystemModule,
};
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateModuleId(pub ModuleId);

impl fmt::Display for DuplicateModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "duplicate module id: {}", self.0)
    }
}

impl std::error::Error for DuplicateModuleId {}

#[derive(Debug, Clone, Default)]
pub struct ModuleRegistry {
    modules: Vec<Arc<dyn IslandModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.modules.push(Arc::new(SystemModule));
        registry.modules.push(Arc::new(MediaModule));
        registry.modules.push(Arc::new(AlertModule));
        registry.modules.push(Arc::new(PrayerModule));
        registry
    }

    pub fn register(
        &mut self,
        module: Arc<dyn IslandModule>,
    ) -> Result<(), DuplicateModuleId> {
        let id = ModuleId::new(module.id());
        if self.get(id).is_some() {
            return Err(DuplicateModuleId(id));
        }
        self.modules.push(module);
        Ok(())
    }

    pub fn get(&self, id: ModuleId) -> Option<&Arc<dyn IslandModule>> {
        self.modules
            .iter()
            .find(|module| module.id() == id.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn IslandModule>> {
        self.modules.iter()
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ActiveModule;
    use crate::view_engine::{ModuleTiers, Priority, SlotContext};

    #[derive(Debug)]
    struct DummyModule {
        id: &'static str,
    }

    impl IslandModule for DummyModule {
        fn id(&self) -> &'static str {
            self.id
        }

        fn name(&self) -> &'static str {
            "Dummy"
        }

        fn module_type(&self) -> ActiveModule {
            ActiveModule::System
        }

        fn tiers(&self) -> ModuleTiers {
            ModuleTiers::ALL
        }

        fn priority(&self, _ctx: &SlotContext) -> Priority {
            Priority::INACTIVE
        }
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let mut registry = ModuleRegistry::new();
        registry
            .register(Arc::new(DummyModule { id: "dummy" }))
            .expect("first registration succeeds");

        let error = registry
            .register(Arc::new(DummyModule { id: "dummy" }))
            .expect_err("duplicate registration fails");

        assert_eq!(error, DuplicateModuleId(ModuleId::new("dummy")));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn lookup_does_not_depend_on_registration_order() {
        let mut registry = ModuleRegistry::new();
        registry
            .register(Arc::new(DummyModule { id: "first" }))
            .expect("register first");
        registry
            .register(Arc::new(DummyModule { id: "second" }))
            .expect("register second");

        assert_eq!(registry.get(ModuleId::new("second")).map(|m| m.id()), Some("second"));
        assert_eq!(registry.get(ModuleId::new("first")).map(|m| m.id()), Some("first"));
        assert!(registry.get(ModuleId::new("missing")).is_none());
    }

    #[test]
    fn default_registry_order_is_deterministic() {
        let registry = ModuleRegistry::with_defaults();
        let ids: Vec<&str> = registry.iter().map(|module| module.id()).collect();

        assert_eq!(ids, ["system", "media", "alert", "prayer"]);
    }
}
