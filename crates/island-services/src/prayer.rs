use chrono::Utc;
use island_core::prayer::{latest_due_alert, PrayerAlert, PrayerSettings};
use tokio::sync::{mpsc::UnboundedSender, watch};
use tokio::time::{sleep, Duration};

const POLL_INTERVAL: Duration = Duration::from_secs(20);
const DUE_GRACE_MINUTES: i64 = 2;

pub struct PrayerService;

impl PrayerService {
    pub fn start(settings_rx: watch::Receiver<PrayerSettings>, sender: UnboundedSender<PrayerAlert>) {
        tokio::spawn(async move {
            Self::run(settings_rx, sender).await;
        });
    }

    async fn run(
        mut settings_rx: watch::Receiver<PrayerSettings>,
        sender: UnboundedSender<PrayerAlert>,
    ) {
        let mut settings = *settings_rx.borrow();
        let mut last_sent: Option<PrayerAlert> = None;

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
                if sender.send(alert).is_err() {
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
            }
        }
    }
}
