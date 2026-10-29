use crate::service::{DomainEvent, Service, ServiceContext, ServiceError, ServiceHandle};
use futures_lite::StreamExt;
use tracing::{debug, error, info};
use zbus::MatchRule;
use zbus::MessageStream;

pub struct DndService;

impl DndService {
    pub async fn fetch_state() -> bool {
        if let Ok(conn) = zbus::Connection::session().await {
            if let Ok(msg) = conn
                .call_method(
                    Some("org.freedesktop.Notifications"),
                    "/org/freedesktop/Notifications",
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.Notifications", "Inhibited"),
                )
                .await
            {
                if let Ok(val) = msg.body().deserialize::<zbus::zvariant::OwnedValue>() {
                    if let zbus::zvariant::Value::Bool(b) = &*val {
                        return *b;
                    }
                }
            }
        }
        false
    }

    pub async fn toggle_dnd() {
        if let Ok(conn) = zbus::Connection::session().await {
            let call_res = conn
                .call_method(
                    Some("org.kde.kglobalaccel"),
                    "/component/plasmashell",
                    Some("org.kde.kglobalaccel.Component"),
                    "invokeShortcut",
                    &("toggle do not disturb",),
                )
                .await;

            if let Err(e) = call_res {
                debug!("KGlobalAccel shortcut failed for DND: {}", e);
            }
        }
    }
}

impl Service for DndService {
    fn name(&self) -> &'static str {
        "dnd"
    }

    fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle, ServiceError> {
        let shutdown = ctx.shutdown_signal().clone();
        let task_ctx = ctx.clone();
        let task_shutdown = shutdown.clone();
        let join = ctx.spawn(async move {
            info!("DND Service starting via Session D-Bus (zero polling)");
            let initial = DndService::fetch_state().await;
            let _ = task_ctx.emit(DomainEvent::Dnd(initial));

            let conn = match zbus::Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to connect to session bus for DND: {}", e);
                    return;
                }
            };

            let prop_rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .interface("org.freedesktop.DBus.Properties")
                .and_then(|b| b.member("PropertiesChanged"))
                .and_then(|b| b.path("/org/freedesktop/Notifications"))
                .map(|b| b.build());

            let mut stream = match prop_rule {
                Ok(rule) => match MessageStream::for_match_rule(rule, &conn, None).await {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to subscribe to DND PropertiesChanged: {}", e);
                        return;
                    }
                },
                Err(e) => {
                    error!("Failed to build DND match rule: {}", e);
                    return;
                }
            };

            info!("DND Service subscribed to Notifications Inhibited property");

            loop {
                tokio::select! {
                    _ = task_shutdown.cancelled() => break,
                    message = stream.next() => {
                        if message.is_none() {
                            break;
                        }
                        let state = DndService::fetch_state().await;
                        debug!("DND status changed via event: {}", state);
                        if !task_ctx.emit(DomainEvent::Dnd(state)) {
                            break;
                        }
                    }
                }
            }
        });
        Ok(ServiceHandle::new(self.name(), shutdown, join))
    }
}
