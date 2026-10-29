use crate::module::{ModuleId, Priority};

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
