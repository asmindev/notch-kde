use crate::service::{DomainEvent, Service, ServiceContext, ServiceError, ServiceHandle};
use chrono::Utc;
use island_core::prayer::{PrayerSettings, latest_due_alert};
use tokio::sync::watch;
use tokio::time::{Duration, sleep};

const POLL_INTERVAL: Duration = Duration::from_secs(20);
const DUE_GRACE_MINUTES: i64 = 2;

pub struct PrayerService {
    settings_rx: watch::Receiver<PrayerSettings>,
}

impl PrayerService {
    pub fn new(settings_rx: watch::Receiver<PrayerSettings>) -> Self {
        Self { settings_rx }
    }
}

impl Service for PrayerService {
    fn name(&self) -> &'static str {
        "prayer"
    }

    fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle, ServiceError> {
        let shutdown = ctx.shutdown_signal().clone();
        let task_ctx = ctx.clone();
        let task_shutdown = shutdown.clone();
        let mut settings_rx = self.settings_rx.clone();
        let join = ctx.spawn(async move {
            let mut settings = *settings_rx.borrow();
            let mut last_sent = None;

            loop {
                if *settings_rx.borrow() != settings {
                    settings = *settings_rx.borrow();
                    last_sent = latest_due_alert(&settings, Utc::now(), DUE_GRACE_MINUTES);
                }

                if settings.enabled
                    && let Some(alert) = latest_due_alert(&settings, Utc::now(), DUE_GRACE_MINUTES)
                    && last_sent.as_ref() != Some(&alert)
                {
                    last_sent = Some(alert.clone());
                    if !task_ctx.emit(DomainEvent::Prayer(alert)) {
                        break;
                    }
                }

                tokio::select! {
                    _ = sleep(POLL_INTERVAL) => {}
                    changed = settings_rx.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                    _ = task_shutdown.cancelled() => break,
                }
            }
        });
        Ok(ServiceHandle::new(self.name(), shutdown, join))
    }
}
