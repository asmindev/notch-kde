use chrono::{Local, Timelike};
use island_core::models::TimeInfo;
use tokio::sync::mpsc::UnboundedSender;
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

    pub fn start(sender: UnboundedSender<TimeInfo>) {
        tokio::spawn(async move {
            info!("Clock service started (aligned to minute boundaries)");
            loop {
                let now = Local::now();
                let time_info = TimeInfo {
                    time_str: now.format("%H:%M").to_string(),
                    date_str: now.format("%a, %d %b").to_string(),
                };
                if sender.send(time_info).is_err() {
                    break;
                }

                // Sleep directly until the start of next minute: 0% CPU during the minute!
                let sec = now.second();
                let sleep_secs = 60 - sec;
                tokio::time::sleep(Duration::from_secs(sleep_secs.max(1) as u64)).await;
            }
            info!("Clock service stopped");
        });
    }
}
