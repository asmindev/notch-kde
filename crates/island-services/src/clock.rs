use crate::service::{DomainEvent, Service, ServiceContext, ServiceError, ServiceHandle};
use chrono::{Local, Timelike};
use island_core::models::TimeInfo;
use tokio::time::Duration;
use tracing::info;

pub struct ClockService;

impl ClockService {
    pub fn get_current_time() -> TimeInfo {
        let now = Local::now();
        TimeInfo {
            time_str: now.format("%H:%M").to_string(),
            date_str: now.format("%a, %d %b").to_string(),
        }
    }
}

impl Service for ClockService {
    fn name(&self) -> &'static str {
        "clock"
    }

    fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle, ServiceError> {
        let shutdown = ctx.shutdown_signal().clone();
        let task_ctx = ctx.clone();
        let task_shutdown = shutdown.clone();
        let join = ctx.spawn(async move {
            info!("Clock service started (aligned to minute boundaries)");
            loop {
                if !task_ctx.emit(DomainEvent::Tick(ClockService::get_current_time())) {
                    break;
                }

                let sec = Local::now().second();
                let sleep_secs = 60 - sec;
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(sleep_secs.max(1) as u64)) => {}
                    _ = task_shutdown.cancelled() => break,
                }
            }
            info!("Clock service stopped");
        });
        Ok(ServiceHandle::new(self.name(), shutdown, join))
    }
}
