use crate::service::{DomainEvent, Service, ServiceContext, ServiceError, ServiceHandle};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use tracing::{debug, error, info, warn};

pub struct CavaService {
    config_path: PathBuf,
}

impl CavaService {
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }
}

impl Service for CavaService {
    fn name(&self) -> &'static str {
        "cava"
    }

    fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle, ServiceError> {
        let shutdown = ctx.shutdown_signal().clone();
        let task_shutdown = shutdown.clone();
        let task_ctx = ctx.clone();
        let config_path = self.config_path.clone();
        let join = ctx.spawn_blocking(move || {
            run_cava(task_shutdown.clone(), task_ctx, config_path);
            info!("CavaService stopped");
        });
        Ok(ServiceHandle::new(self.name(), shutdown, join))
    }
}

fn run_cava(shutdown: crate::service::ShutdownSignal, ctx: ServiceContext, config_path: PathBuf) {
    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

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
        error!(service = "cava", "Failed to write cava config: {}", e);
        return;
    }

    info!(
        service = "cava",
        "Spawning cava with config: {:?}", config_path
    );
    let mut child: Option<Child> = Command::new("cava")
        .arg("-p")
        .arg(&config_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok();

    let Some(child_process) = child.as_mut() else {
        warn!(
            service = "cava",
            "Could not start CAVA; audio visualizer disabled."
        );
        return;
    };

    let pid = Arc::new(AtomicU32::new(child_process.id()));
    let done = Arc::new(AtomicBool::new(false));
    let watcher_pid = pid.clone();
    let watcher_done = done.clone();
    let watcher_shutdown = shutdown.clone();
    let watcher = std::thread::spawn(move || {
        while !watcher_shutdown.is_shutdown() && !watcher_done.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if watcher_shutdown.is_shutdown() {
            let pid = watcher_pid.load(Ordering::SeqCst);
            if pid > 0 {
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
            }
        }
    });

    let mut stdout = child_process.stdout.take().expect("cava stdout is piped");
    let mut buf = [0u8; 8];
    let mut was_silent = false;

    loop {
        match stdout.read_exact(&mut buf) {
            Ok(_) => {
                let is_silent = buf.iter().all(|&b| b == 0);
                if is_silent {
                    if !was_silent {
                        was_silent = true;
                        let _ = ctx.emit(DomainEvent::Spectrum([0.0f32; 8]));
                    }
                    continue;
                }

                was_silent = false;
                let mut bars = [0.0f32; 8];
                for (bar, value) in bars.iter_mut().zip(buf.iter()) {
                    *bar = (*value as f32) / 255.0;
                }
                if !ctx.emit(DomainEvent::Spectrum(bars)) {
                    break;
                }
            }
            Err(e) => {
                debug!(service = "cava", "Cava stream ended: {}", e);
                break;
            }
        }
    }

    done.store(true, Ordering::SeqCst);
    if let Some(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
    }
    pid.store(0, Ordering::SeqCst);
    let _ = watcher.join();
}
