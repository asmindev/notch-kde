use crate::config::WindowSpec;
use island_platform::kde::ScreenGeometry;
use island_platform::{KdeManager, PlatformWindow};
use island_ui::MainWindow;
use tracing::{info, warn};

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

    pub fn setup_window_rules(window_spec: &WindowSpec) {
        let geom = KdeManager::get_primary_screen_geometry().unwrap_or_else(|| {
            warn!("kscreen-doctor/qdbus unavailable; using fallback screen geometry");
            ScreenGeometry::fallback()
        });
        let window_x = window_spec.window_x(&geom);
        let window_y = geom.y;
        match PlatformWindow::ensure_kwin_rules(window_x, window_y, geom.screen_index) {
            Ok(true) => {
                info!("Waiting for KWin to apply the updated window rule...");
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            Ok(false) => {}
            Err(err) => warn!("Failed to ensure KWin window rule: {err}"),
        }
    }

    pub fn create_window() -> Result<MainWindow, slint::PlatformError> {
        info!("Selecting Slint femtovg (OpenGL) backend...");
        match slint::BackendSelector::new()
            .renderer_name("femtovg".to_string())
            .select()
        {
            Ok(()) => info!("Using Slint femtovg (OpenGL) backend"),
            Err(err) => {
                warn!("Femtovg backend unavailable ({err}); falling back to default backend")
            }
        }

        info!("Creating Slint MainWindow...");
        MainWindow::new()
    }
}
