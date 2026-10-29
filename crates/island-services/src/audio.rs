use crate::service::{DomainEvent, Service, ServiceContext, ServiceError, ServiceHandle};
use tokio::io::AsyncBufReadExt;
use tracing::{debug, error, info};

pub struct AudioService;

impl AudioService {
    pub fn check_muted_sync() -> bool {
        if let Ok(output) = std::process::Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
        {
            let out = String::from_utf8_lossy(&output.stdout);
            return out.to_lowercase().contains("[muted]");
        }
        false
    }

    pub async fn toggle_mute() {
        if let Ok(conn) = zbus::Connection::session().await {
            let call_res = conn
                .call_method(
                    Some("org.kde.kglobalaccel"),
                    "/component/kmix",
                    Some("org.kde.kglobalaccel.Component"),
                    "invokeShortcut",
                    &("mute",),
                )
                .await;

            if call_res.is_err() {
                let _ = tokio::process::Command::new("wpctl")
                    .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                    .status()
                    .await;
            }
        } else {
            let _ = tokio::process::Command::new("wpctl")
                .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                .status()
                .await;
        }
    }
}

impl Service for AudioService {
    fn name(&self) -> &'static str {
        "audio"
    }

    fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle, ServiceError> {
        let shutdown = ctx.shutdown_signal().clone();
        let task_ctx = ctx.clone();
        let task_shutdown = shutdown.clone();
        let join = ctx.spawn(async move {
            info!("Audio Service starting via pactl subscribe event stream (zero polling)");
            let initial = AudioService::check_muted_sync();
            let _ = task_ctx.emit(DomainEvent::Muted(initial));

            let mut child = match tokio::process::Command::new("pactl")
                .arg("subscribe")
                .stdout(std::process::Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to spawn pactl subscribe: {}", e);
                    return;
                }
            };

            if let Some(stdout) = child.stdout.take() {
                let reader = tokio::io::BufReader::new(stdout);
                let mut lines = reader.lines();
                let mut last_muted = initial;

                loop {
                    tokio::select! {
                        _ = task_shutdown.cancelled() => break,
                        line = lines.next_line() => {
                            let Ok(Some(line)) = line else { break };
                            if line.contains("sink") || line.contains("server") || line.contains("card") {
                                let muted = AudioService::check_muted_sync();
                                if muted != last_muted {
                                    debug!("Audio mute changed via event: {}", muted);
                                    last_muted = muted;
                                    if !task_ctx.emit(DomainEvent::Muted(muted)) {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let _ = child.kill().await;
            let _ = child.wait().await;
            info!("Audio Service stopped");
        });
        Ok(ServiceHandle::new(self.name(), shutdown, join))
    }
}
