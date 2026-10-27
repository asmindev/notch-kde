use island_platform::{KdeManager, PlatformWindow};
use island_ui::MainWindow;
use tracing::info;

pub struct Bootstrap;

impl Bootstrap {
    pub fn init_logging() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,island=debug")),
            )
            .try_init();
        info!("KDE Dynamic Island initializing...");
    }

    pub fn setup_window_rules() {
        let geom = KdeManager::get_primary_screen_geometry();
        let island_w = 480;
        let window_x = geom.x + if geom.width > island_w {
            ((geom.width - island_w) / 2) as i32
        } else {
            0
        };
        let window_y = geom.y;
        let _ = PlatformWindow::ensure_kwin_rules(window_x, window_y, geom.screen_index);
    }

    pub fn create_window() -> Result<MainWindow, slint::PlatformError> {
        info!("Creating Slint MainWindow...");
        MainWindow::new()
    }
}
