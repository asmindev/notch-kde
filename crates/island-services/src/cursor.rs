use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

use island_platform::KdeManager;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

pub struct CursorService;

struct CursorBridge {
    geom: Arc<Mutex<island_platform::ScreenGeometry>>,
    tx: UnboundedSender<(f32, f32)>,
}

#[zbus::interface(name = "dev.notchde.Island.Cursor")]
impl CursorBridge {
    async fn cursor_moved(&self, x: f64, y: f64) {
        let geom = *self.geom.lock().unwrap();
        let width = geom.width.max(1) as f64;
        let height = geom.height.max(1) as f64;
        let nx = ((x - geom.x as f64) / width * 2.0 - 1.0) as f32;
        let ny = ((y - geom.y as f64) / height * 2.0 - 1.0) as f32;
        let _ = self.tx.send((nx.clamp(-1.0, 1.0), ny.clamp(-1.0, 1.0)));
    }
}

impl CursorService {
    pub async fn serve(
        geom: Arc<Mutex<island_platform::ScreenGeometry>>,
        tx: UnboundedSender<(f32, f32)>,
    ) {
        let builder = match zbus::connection::Builder::session() {
            Ok(builder) => builder,
            Err(err) => {
                warn!("Could not open session bus for cursor bridge: {err}");
                return;
            }
        };
        let builder = match builder.name("dev.notchde.Island") {
            Ok(builder) => builder,
            Err(err) => {
                warn!("Could not reserve cursor D-Bus name: {err}");
                return;
            }
        };
        let builder = match builder.serve_at(
            "/dev/notchde/Island/Cursor",
            CursorBridge { geom, tx },
        ) {
            Ok(builder) => builder,
            Err(err) => {
                warn!("Could not register cursor D-Bus object: {err}");
                return;
            }
        };
        match builder.build().await {
            Ok(_connection) => {
                info!("Bloub global cursor D-Bus bridge ready");
                std::future::pending::<()>().await;
            }
            Err(err) => warn!("Could not start cursor D-Bus bridge: {err}"),
        }
    }

    pub fn install_kwin_script() -> bool {
        if !is_kde() {
            return false;
        }
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return false;
        };
        let dir = home.join(".local/share/kwin/scripts/notchde-cursor");
        let code_dir = dir.join("contents/code");
        if let Err(err) = std::fs::create_dir_all(&code_dir) {
            warn!("Could not create KWin cursor script directory: {err}");
            return false;
        }

        let script = r#"// notchde global cursor bridge
var notchdeLastCursorSend = 0;

function notchdeSendCursor(force) {
    var now = Date.now();
    if (!force && now - notchdeLastCursorSend < 16) {
        return;
    }
    notchdeLastCursorSend = now;
    var p = workspace.cursorPos;
    callDBus(
        "dev.notchde.Island",
        "/dev/notchde/Island/Cursor",
        "dev.notchde.Island.Cursor",
        "CursorMoved",
        p.x,
        p.y
    );
}

if (typeof workspace !== "undefined" && workspace.cursorPosChanged) {
    workspace.cursorPosChanged.connect(function () { notchdeSendCursor(false); });
    notchdeSendCursor(true);
}
"#;

        let metadata = r#"{
  "KPlugin": {
    "Id": "notchde-cursor",
    "Name": "notchde Global Cursor Bridge",
    "Description": "Feeds the global KWin cursor position to the notchde Bloub pet.",
    "Icon": "preferences-system",
    "License": "MIT",
    "Version": "1.0"
  },
  "X-Plasma-API": "javascript",
  "X-Plasma-MainScript": "code/main.js"
}
"#;

        if let Err(err) = write_if_changed(&code_dir.join("main.js"), script) {
            warn!("Could not write KWin cursor script: {err}");
            return false;
        }
        if let Err(err) = write_if_changed(&dir.join("metadata.json"), metadata) {
            warn!("Could not write KWin cursor metadata: {err}");
            return false;
        }

        let configured = Command::new("kwriteconfig6")
            .args([
                "--file",
                "kwinrc",
                "--group",
                "Plugins",
                "--key",
                "notchde-cursorEnabled",
                "true",
            ])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
            || Command::new("kwriteconfig5")
                .args([
                    "--file",
                    "kwinrc",
                    "--group",
                    "Plugins",
                    "--key",
                    "notchde-cursorEnabled",
                    "true",
                ])
                .status()
                .map(|status| status.success())
                .unwrap_or(false);

        if configured {
            let reconfigured = Command::new("qdbus6")
                .args(["org.kde.KWin", "/KWin", "org.kde.KWin.reconfigure"])
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
                || Command::new("qdbus")
                    .args(["org.kde.KWin", "/KWin", "org.kde.KWin.reconfigure"])
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false)
                || Command::new("dbus-send")
                    .args([
                        "--session",
                        "--dest=org.kde.KWin",
                        "/KWin",
                        "org.kde.KWin.reconfigure",
                    ])
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false);
            if !reconfigured {
                warn!("KWin cursor script is enabled but KWin did not reload it");
            }
            info!("Installed KWin global cursor script");
        } else {
            warn!("Could not enable the KWin cursor script; global gaze is disabled");
        }

        configured
    }
}

fn write_if_changed(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    if let Ok(existing) = std::fs::read_to_string(path)
        && existing == contents
    {
        return Ok(());
    }
    std::fs::write(path, contents)
}

fn is_kde() -> bool {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let session = std::env::var("KDE_FULL_SESSION").unwrap_or_default();
    desktop.to_ascii_uppercase().contains("KDE")
        || desktop.to_ascii_uppercase().contains("PLASMA")
        || !session.is_empty()
}

pub fn primary_screen_geometry() -> Option<island_platform::ScreenGeometry> {
    KdeManager::try_get_primary_screen_geometry()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge(geom: island_platform::ScreenGeometry) -> (CursorBridge, tokio::sync::mpsc::UnboundedReceiver<(f32, f32)>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (
            CursorBridge {
                geom: Arc::new(Mutex::new(geom)),
                tx,
            },
            rx,
        )
    }

    #[test]
    fn normalizes_cursor_against_primary_screen() {
        let geom = island_platform::ScreenGeometry {
            x: 1920,
            y: 0,
            width: 1920,
            height: 1080,
            screen_index: 1,
        };
        let (bridge, mut rx) = bridge(geom);

        futures_lite::future::block_on(bridge.cursor_moved(1920.0, 0.0));
        assert_eq!(rx.try_recv(), Ok((-1.0, -1.0)));

        futures_lite::future::block_on(bridge.cursor_moved(2880.0, 540.0));
        assert_eq!(rx.try_recv(), Ok((0.0, 0.0)));

        futures_lite::future::block_on(bridge.cursor_moved(3840.0, 1080.0));
        assert_eq!(rx.try_recv(), Ok((1.0, 1.0)));
    }

    #[test]
    fn follows_screen_geometry_updates() {
        let (bridge, mut rx) = bridge(island_platform::ScreenGeometry {
            x: 0,
            y: 0,
            width: 1000,
            height: 1000,
            screen_index: 0,
        });

        futures_lite::future::block_on(bridge.cursor_moved(500.0, 500.0));
        assert_eq!(rx.try_recv(), Ok((0.0, 0.0)));

        *bridge.geom.lock().unwrap() = island_platform::ScreenGeometry {
            x: 1000,
            y: 0,
            width: 1000,
            height: 1000,
            screen_index: 1,
        };
        futures_lite::future::block_on(bridge.cursor_moved(1500.0, 500.0));
        assert_eq!(rx.try_recv(), Ok((0.0, 0.0)));
    }
}
