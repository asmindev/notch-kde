use island_platform::{KdeManager, PlatformWindow};
use island_services::CursorService;
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

    pub fn setup_window_rules() {
        let geom = KdeManager::get_primary_screen_geometry();
        let island_w = 480;
        let window_x = geom.x + if geom.width > island_w {
            ((geom.width - island_w) / 2) as i32
        } else {
            0
        };
        let window_y = geom.y;
        match PlatformWindow::ensure_kwin_rules(window_x, window_y, geom.screen_index) {
            Ok(true) => {
                info!("Waiting for KWin to apply the updated window rule...");
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            Ok(false) => {}
            Err(err) => warn!("Failed to ensure KWin window rule: {err}"),
        }
        CursorService::install_kwin_script();
    }

    pub fn create_window() -> Result<MainWindow, slint::PlatformError> {
        info!("Selecting Slint WGPU backend...");
        match slint::BackendSelector::new()
            .require_wgpu_30(slint::wgpu_30::WGPUConfiguration::default())
            .select()
        {
            Ok(()) => info!("Using Slint WGPU backend"),
            Err(err) => warn!("WGPU backend unavailable ({err}); falling back to default backend"),
        }

        info!("Creating Slint MainWindow...");
        MainWindow::new()
    }
}
