use tracing::info;

pub struct NotificationService;

impl NotificationService {
    pub fn new() -> Self {
        Self
    }

    pub fn start() {
        info!("Notification service ready");
    }
}
