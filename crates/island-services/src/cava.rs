use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info, warn};

pub struct CavaService {
    running: Arc<AtomicBool>,
}

impl CavaService {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self, sender: UnboundedSender<[f32; 8]>) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let running = Arc::clone(&self.running);

        std::thread::spawn(move || {
            let config_dir = std::env::var("XDG_CONFIG_HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                    std::path::PathBuf::from(home).join(".config")
                })
                .join("island-cava");
            let _ = std::fs::create_dir_all(&config_dir);
            let config_path = config_dir.join("cava.conf");

            let config_content = r#"
[general]
bars = 8
framerate = 30
autosens = 1

[input]
method = pulse

[output]
method = raw
raw_target = /dev/stdout
data_format = binary
bit_format = 8bit
"#;
            if let Err(e) = std::fs::write(&config_path, config_content) {
                error!("Failed to write cava config: {}", e);
                return;
            }

            info!("Spawning cava with config: {:?}", config_path);
            let mut child: Option<Child> = Command::new("cava")
                .arg("-p")
                .arg(&config_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .ok();

            if child.is_none() {
                warn!("Could not start CAVA; audio visualizer disabled.");
                return;
            }

            let mut stdout = child.as_mut().unwrap().stdout.take().unwrap();
            let mut buf = [0u8; 8];
            let mut was_silent = false;

            while running.load(Ordering::Relaxed) {
                match stdout.read_exact(&mut buf) {
                    Ok(_) => {
                        let is_silent = buf.iter().all(|&b| b == 0);
                        if is_silent {
                            if !was_silent {
                                was_silent = true;
                                // Send 0.0 bars once so visualizer smoothly returns to baseline
                                let _ = sender.send([0.0f32; 8]);
                            }
                            // Silent: do not send redundant events to Slint (saves CPU)
                            continue;
                        }

                        was_silent = false;
                        let mut bars = [0.0f32; 8];
                        for i in 0..8 {
                            bars[i] = (buf[i] as f32) / 255.0;
                        }
                        if sender.send(bars).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        debug!("Cava stream ended: {}", e);
                        break;
                    }
                }
            }

            if let Some(mut c) = child {
                let _ = c.kill();
            }
            info!("CavaService stopped");
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
