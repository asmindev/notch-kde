use island_platform::kde::ScreenGeometry;
use island_platform::overlap::Rect;
use island_ui::{MainWindow, ThemeMetrics};
use serde::{Deserialize, Serialize};
use slint::ComponentHandle;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Deployment/internal geometry shared by Slint, window placement, and input regions.
///
/// `ThemeMetrics` geometry properties are populated from this spec before the
/// window is shown; non-geometry visual tokens remain in Slint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSpec {
    pub window_width: u32,
    pub window_height: u32,
    pub compact_width: u32,
    pub compact_alert_width: u32,
    pub compact_height: u32,
    pub compact_touch_height: u32,
    pub wide_width: u32,
    pub wide_height: u32,
    pub wide_touch_height: u32,
    pub expand_width: u32,
    pub expand_height: u32,
    pub expand_touch_height: u32,
    pub hidden_touch_height: u32,
    pub welcome_height: u32,
    pub bottom_bar_height: u32,
    pub ear_size: u32,
    pub compact_ear_size: u32,
    pub wide_ear_size: u32,
}

impl Default for WindowSpec {
    fn default() -> Self {
        Self {
            window_width: 480,
            window_height: 208,
            compact_width: 156,
            compact_alert_width: 228,
            compact_height: 30,
            compact_touch_height: 42,
            wide_width: 390,
            wide_height: 56,
            wide_touch_height: 98,
            expand_width: 440,
            expand_height: 152,
            expand_touch_height: 196,
            hidden_touch_height: 28,
            welcome_height: 78,
            bottom_bar_height: 32,
            ear_size: 10,
            compact_ear_size: 5,
            wide_ear_size: 10,
        }
    }
}

impl WindowSpec {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_dimension("window_width", self.window_width, 320, 7680)?;
        self.validate_dimension("window_height", self.window_height, 64, 4320)?;
        self.validate_dimension("compact_width", self.compact_width, 40, self.window_width)?;
        self.validate_dimension(
            "compact_alert_width",
            self.compact_alert_width,
            self.compact_width,
            self.window_width,
        )?;
        self.validate_dimension("compact_height", self.compact_height, 1, self.window_height)?;
        self.validate_dimension(
            "compact_touch_height",
            self.compact_touch_height,
            self.compact_height,
            self.window_height,
        )?;
        self.validate_dimension(
            "wide_width",
            self.wide_width,
            self.compact_alert_width,
            self.window_width,
        )?;
        self.validate_dimension("wide_height", self.wide_height, 1, self.window_height)?;
        self.validate_dimension(
            "wide_touch_height",
            self.wide_touch_height,
            self.wide_height,
            self.window_height,
        )?;
        self.validate_dimension(
            "expand_width",
            self.expand_width,
            self.wide_width,
            self.window_width,
        )?;
        self.validate_dimension("expand_height", self.expand_height, 1, self.window_height)?;
        self.validate_dimension(
            "expand_touch_height",
            self.expand_touch_height,
            self.expand_height,
            self.window_height,
        )?;
        self.validate_dimension(
            "hidden_touch_height",
            self.hidden_touch_height,
            1,
            self.window_height,
        )?;
        self.validate_dimension("welcome_height", self.welcome_height, 1, self.window_height)?;
        self.validate_dimension(
            "bottom_bar_height",
            self.bottom_bar_height,
            0,
            self.window_height,
        )?;
        self.validate_dimension("ear_size", self.ear_size, 1, 64)?;
        self.validate_dimension("compact_ear_size", self.compact_ear_size, 1, 64)?;
        self.validate_dimension("wide_ear_size", self.wide_ear_size, 1, 64)?;

        if self.expand_height + self.bottom_bar_height > self.window_height {
            return Err(format!(
                "expand_height + bottom_bar_height ({}) exceeds window_height ({})",
                self.expand_height + self.bottom_bar_height,
                self.window_height
            ));
        }

        Ok(())
    }

    fn validate_dimension(
        &self,
        name: &str,
        value: u32,
        minimum: u32,
        maximum: u32,
    ) -> Result<(), String> {
        if value < minimum || value > maximum {
            return Err(format!(
                "{name} must be between {minimum} and {maximum}, got {value}"
            ));
        }
        Ok(())
    }

    pub fn window_x(&self, geometry: &ScreenGeometry) -> i32 {
        geometry.x + (geometry.width.saturating_sub(self.window_width) / 2) as i32
    }

    pub fn compact_notch_rect(&self, geometry: &ScreenGeometry) -> Rect {
        Rect {
            x: geometry.x
                + if geometry.width > self.compact_width {
                    ((geometry.width - self.compact_width) / 2) as i32
                } else {
                    0
                },
            y: geometry.y,
            w: self.compact_width,
            h: self.compact_height,
        }
    }

    pub fn apply_to_ui(&self, window: &MainWindow) {
        let metrics = window.global::<ThemeMetrics>();
        metrics.set_window_width(self.window_width as f32);
        metrics.set_window_height(self.window_height as f32);
        metrics.set_compact_width(self.compact_width as f32);
        metrics.set_compact_alert_width(self.compact_alert_width as f32);
        metrics.set_compact_height(self.compact_height as f32);
        metrics.set_wide_width(self.wide_width as f32);
        metrics.set_wide_height(self.wide_height as f32);
        metrics.set_expand_width(self.expand_width as f32);
        metrics.set_expand_height(self.expand_height as f32);
        metrics.set_welcome_height(self.welcome_height as f32);
        metrics.set_bottom_bar_height(self.bottom_bar_height as f32);
        metrics.set_ear_size(self.ear_size as f32);
        metrics.set_compact_ear_size(self.compact_ear_size as f32);
        metrics.set_wide_ear_size(self.wide_ear_size as f32);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CavaConfig {
    pub enabled: bool,
    pub config_path: PathBuf,
}

impl Default for CavaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            config_path: PathBuf::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServiceToggles {
    pub clock: bool,
    pub battery: bool,
    pub media: bool,
    pub notifications: bool,
    pub prayer: bool,
    pub dnd: bool,
    pub audio: bool,
    pub screen_saver: bool,
}

impl Default for ServiceToggles {
    fn default() -> Self {
        Self {
            clock: true,
            battery: true,
            media: true,
            notifications: true,
            prayer: true,
            dnd: true,
            audio: true,
            screen_saver: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub window: WindowSpec,
    pub dodge_enabled: bool,
    pub config_dir: PathBuf,
    pub cava: CavaConfig,
    pub services: ServiceToggles,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window: WindowSpec::default(),
            dodge_enabled: true,
            config_dir: PathBuf::new(),
            cava: CavaConfig::default(),
            services: ServiceToggles::default(),
        }
    }
}

impl AppConfig {
    pub fn settings_path(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    pub fn validate(&self) -> Result<(), String> {
        self.window.validate()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConfigEnv {
    pub home: Option<PathBuf>,
    pub xdg_config_home: Option<PathBuf>,
    pub notchde_config_dir: Option<PathBuf>,
    pub notchde_cava_config: Option<PathBuf>,
    pub island_dodge: Option<String>,
    pub pill_dodge: Option<String>,
}

impl ConfigEnv {
    pub fn from_process() -> Self {
        Self {
            home: std::env::var_os("HOME").map(PathBuf::from),
            xdg_config_home: std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            notchde_config_dir: std::env::var_os("NOTCHDE_CONFIG_DIR").map(PathBuf::from),
            notchde_cava_config: std::env::var_os("NOTCHDE_CAVA_CONFIG").map(PathBuf::from),
            island_dodge: std::env::var("ISLAND_DODGE").ok(),
            pill_dodge: std::env::var("PILL_DODGE").ok(),
        }
    }

    fn config_root(&self) -> PathBuf {
        if let Some(path) = self
            .notchde_config_dir
            .as_ref()
            .filter(|path| !path.as_os_str().is_empty())
        {
            return path.clone();
        }
        self.config_home()
            .map(|path| path.join("notchde"))
            .unwrap_or_else(|| std::env::temp_dir().join("notchde"))
    }

    fn default_cava_config(&self) -> PathBuf {
        self.config_home()
            .map(|path| path.join("island-cava").join("cava.conf"))
            .unwrap_or_else(|| std::env::temp_dir().join("island-cava").join("cava.conf"))
    }

    fn config_home(&self) -> Option<PathBuf> {
        if let Some(path) = self
            .xdg_config_home
            .as_ref()
            .filter(|path| !path.as_os_str().is_empty())
        {
            return Some(path.clone());
        }
        self.home
            .as_ref()
            .filter(|path| !path.as_os_str().is_empty())
            .map(|path| path.join(".config"))
    }

    fn dodge_override(&self) -> Option<bool> {
        let island = self
            .island_dodge
            .as_deref()
            .and_then(|value| parse_bool_override("ISLAND_DODGE", value));
        let pill = self
            .pill_dodge
            .as_deref()
            .and_then(|value| parse_bool_override("PILL_DODGE", value));
        island.or(pill)
    }
}

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load() -> AppConfig {
        Self::load_with(&ConfigEnv::from_process())
    }

    pub fn load_with(env: &ConfigEnv) -> AppConfig {
        let root = env.config_root();
        let config_path = root.join("config.json");
        let mut config = match read_config(&config_path) {
            Ok(Some(config)) => config,
            Ok(None) => AppConfig::default(),
            Err(error) => {
                warn!(path = %config_path.display(), %error, "using default app config");
                AppConfig::default()
            }
        };

        apply_runtime_paths(&mut config, env, &root);
        if let Err(error) = config.validate() {
            warn!(
                path = %config_path.display(),
                %error,
                "invalid app config; using defaults"
            );
            config = AppConfig::default();
            apply_runtime_paths(&mut config, env, &root);
        }

        info!(
            path = %config_path.display(),
            config_dir = %config.config_dir.display(),
            dodge_enabled = config.dodge_enabled,
            cava_enabled = config.cava.enabled,
            "loaded app config"
        );
        config
    }
}

fn read_config(path: &Path) -> Result<Option<AppConfig>, String> {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    serde_json::from_slice(&data)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn apply_runtime_paths(config: &mut AppConfig, env: &ConfigEnv, root: &Path) {
    config.config_dir = env
        .notchde_config_dir
        .clone()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| {
            if config.config_dir.as_os_str().is_empty() {
                root.to_path_buf()
            } else {
                config.config_dir.clone()
            }
        });

    config.cava.config_path = env
        .notchde_cava_config
        .clone()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| {
            if config.cava.config_path.as_os_str().is_empty() {
                env.default_cava_config()
            } else {
                config.cava.config_path.clone()
            }
        });

    if let Some(enabled) = env.dodge_override() {
        config.dodge_enabled = enabled;
    }
}

fn parse_bool_override(name: &str, value: &str) -> Option<bool> {
    match value {
        "0" => Some(false),
        "1" => Some(true),
        other => {
            warn!(
                variable = name,
                value = other,
                "ignoring invalid boolean override"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn temporary_dir() -> PathBuf {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("notchde-config-test-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create temporary config dir");
        path
    }

    fn env_for(dir: &Path) -> ConfigEnv {
        ConfigEnv {
            xdg_config_home: Some(dir.to_path_buf()),
            notchde_config_dir: Some(dir.to_path_buf()),
            home: None,
            ..ConfigEnv::default()
        }
    }

    #[test]
    fn defaults_match_current_behavior() {
        let config = AppConfig::default();
        assert_eq!(config.window.window_width, 480);
        assert_eq!(config.window.compact_width, 156);
        assert_eq!(config.window.wide_width, 390);
        assert_eq!(config.window.expand_width, 440);
        assert!(config.dodge_enabled);
        assert!(config.cava.enabled);
        assert!(config.services.clock);
        assert!(config.services.media);
    }

    #[test]
    fn partial_json_uses_defaults() {
        let config: AppConfig =
            serde_json::from_str(r#"{"dodge_enabled": false}"#).expect("parse partial config");
        assert!(!config.dodge_enabled);
        assert_eq!(config.window, WindowSpec::default());
        assert_eq!(config.services, ServiceToggles::default());
    }

    #[test]
    fn malformed_json_falls_back_to_defaults() {
        let dir = temporary_dir();
        std::fs::write(dir.join("config.json"), b"not json").expect("write config");

        let config = ConfigLoader::load_with(&env_for(&dir));

        assert_eq!(config.window, WindowSpec::default());
        assert_eq!(config.config_dir, dir);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn out_of_range_values_fall_back_to_defaults() {
        let dir = temporary_dir();
        std::fs::write(
            dir.join("config.json"),
            br#"{"dodge_enabled":false,"window":{"compact_width":0}}"#,
        )
        .expect("write config");

        let config = ConfigLoader::load_with(&env_for(&dir));

        assert!(config.dodge_enabled);
        assert_eq!(config.window, WindowSpec::default());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn environment_overrides_only_in_loader_layer() {
        let dir = temporary_dir();
        let mut env = env_for(&dir);
        env.island_dodge = Some("0".to_string());

        let config = ConfigLoader::load_with(&env);

        assert!(!config.dodge_enabled);
        assert_eq!(config.config_dir, dir);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn window_spec_centers_compact_notch() {
        let spec = WindowSpec::default();
        let geometry = ScreenGeometry {
            x: 100,
            y: 20,
            width: 1516,
            height: 947,
            screen_index: 0,
        };

        let rect = spec.compact_notch_rect(&geometry);

        assert_eq!(rect.x, 780);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.w, 156);
        assert_eq!(rect.h, 30);
    }

    #[test]
    fn invalid_dimension_is_rejected() {
        let spec = WindowSpec {
            compact_width: 0,
            ..WindowSpec::default()
        };

        assert!(spec.validate().is_err());
    }
}
