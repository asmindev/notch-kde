pub mod content_router;
pub mod module_ui_registry;
pub mod view_model;

slint::include_modules!();

pub use content_router::ContentRouter;
pub use module_ui_registry::{ModuleUiRegistry, ModuleUiSlot};
pub use view_model::ViewModel;
