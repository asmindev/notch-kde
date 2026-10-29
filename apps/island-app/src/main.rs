mod bootstrap;
mod config;
mod input_region;
mod pet3d;
mod render_stats;
mod runtime;
mod settings_store;

use bootstrap::Bootstrap;
use config::ConfigLoader;
use runtime::AppRuntime;
use settings_store::SettingsStore;

fn main() -> Result<(), slint::PlatformError> {
    // 1. Initialize Tracing logger
    Bootstrap::init_logging();

    // 2. Load deployment config before touching the UI or platform.
    let config = ConfigLoader::load();
    let settings = SettingsStore::load(&config.settings_path());
    let pet_renderer = settings.pet_renderer.unwrap_or(config.pet.renderer);

    // 3. Configure KWin placement and rules
    Bootstrap::setup_window_rules(&config.window);

    // 4. Create Slint Main Window
    let window = Bootstrap::create_window(pet_renderer)?;
    config.window.apply_to_ui(&window);

    // 5. Start Event-driven Runtime
    let runtime = AppRuntime::new(window, config, settings);
    runtime.run()
}
