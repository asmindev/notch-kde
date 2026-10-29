use crate::service::{DomainEvent, Service, ServiceContext, ServiceError, ServiceHandle};
use futures_lite::StreamExt;
use tracing::{debug, error, info};
use zbus::Connection;
use zbus::MatchRule;
use zbus::MessageStream;

pub struct ScreenSaverService;

impl Service for ScreenSaverService {
    fn name(&self) -> &'static str {
        "screensaver"
    }

    fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle, ServiceError> {
        let shutdown = ctx.shutdown_signal().clone();
        let task_ctx = ctx.clone();
        let task_shutdown = shutdown.clone();
        let join = ctx.spawn(async move {
            info!("ScreenSaver/Lockscreen listener starting");
            let conn = match Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    error!(
                        "Failed to connect to D-Bus session bus for screensaver: {}",
                        e
                    );
                    return;
                }
            };

            let ss_rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .interface("org.freedesktop.ScreenSaver")
                .and_then(|b| b.member("ActiveChanged"))
                .map(|b| b.build());

            let mut ss_stream = match ss_rule {
                Ok(rule) => MessageStream::for_match_rule(rule, &conn, None).await.ok(),
                Err(e) => {
                    error!("Failed to create ScreenSaver match rule: {}", e);
                    None
                }
            };

            info!("Subscribed to ScreenSaver ActiveChanged D-Bus signals");

            loop {
                tokio::select! {
                    _ = task_shutdown.cancelled() => break,
                    message = async {
                        match &mut ss_stream {
                            Some(s) => s.next().await,
                            None => std::future::pending().await,
                        }
                    } => {
                        let Some(Ok(msg)) = message else { break };
                        if let Ok(is_locked) = msg.body().deserialize::<bool>() {
                            debug!("ScreenSaver ActiveChanged: is_locked = {}", is_locked);
                            if !is_locked {
                                info!("Screen unlocked! Emitting Welcome event");
                                if !task_ctx.emit(DomainEvent::ScreenUnlocked) {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });
        Ok(ServiceHandle::new(self.name(), shutdown, join))
    }
}
