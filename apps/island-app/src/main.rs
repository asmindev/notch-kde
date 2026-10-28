mod bootstrap;
mod input_region;
mod pet3d;
mod runtime;
mod settings_store;

use bootstrap::Bootstrap;
use runtime::AppRuntime;

fn main() -> Result<(), slint::PlatformError> {
    // 1. Initialize Tracing logger
    Bootstrap::init_logging();

    // 2. Configure KWin placement and rules
    Bootstrap::setup_window_rules();

    // 3. Create Slint Main Window
    let window = Bootstrap::create_window()?;

    // 4. Start Event-driven Runtime
    let runtime = AppRuntime::new(window);
    runtime.run()
}
