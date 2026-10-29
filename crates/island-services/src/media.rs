use crate::service::{DomainEvent, Service, ServiceContext, ServiceError, ServiceHandle};
use futures_lite::StreamExt;
use island_core::models::{MediaSource, MediaTrack, MediaUpdate};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};
use zbus::Connection;
use zbus::MatchRule;
use zbus::MessageStream;
use zbus::zvariant::Value;

#[derive(Default)]
struct MediaState {
    active_player: Option<String>,
    events: Option<UnboundedSender<DomainEvent>>,
}

#[derive(Clone, Default)]
pub struct MediaService {
    state: Arc<RwLock<MediaState>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCandidate {
    pub bus_name: String,
    pub identity: String,
    pub status: String,
    pub has_metadata: bool,
    pub title: String,
    pub artist: String,
}

pub fn select_target_player<'a>(
    candidates: &'a [PlayerCandidate],
    current_active: Option<&str>,
) -> Option<&'a PlayerCandidate> {
    if candidates.is_empty() {
        return None;
    }

    // Rule 1: A player is "Playing"
    // If the currently active player is among the playing ones, stick with it (stability)
    if let Some(active) = current_active {
        if let Some(cand) = candidates
            .iter()
            .find(|c| c.bus_name == active && c.status.eq_ignore_ascii_case("playing"))
        {
            return Some(cand);
        }
    }

    // Otherwise, pick the first player that is "Playing"
    if let Some(cand) = candidates
        .iter()
        .find(|c| c.status.eq_ignore_ascii_case("playing"))
    {
        return Some(cand);
    }

    // Rule 2: No player is playing.
    // If current_active is still present and has metadata, keep it (e.g. paused Spotify)
    if let Some(active) = current_active {
        if let Some(cand) = candidates
            .iter()
            .find(|c| c.bus_name == active && c.has_metadata)
        {
            return Some(cand);
        }
    }

    // Rule 3: Pick the first player with metadata
    if let Some(cand) = candidates
        .iter()
        .find(|c| c.has_metadata && !c.title.is_empty())
    {
        return Some(cand);
    }

    // Rule 4: Fallback to the first candidate
    candidates.first()
}

impl MediaService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn play_pause(&self) {
        if let Ok(conn) = Connection::session().await {
            let _ = self.call_player_method(&conn, "PlayPause").await;
        }
    }

    pub async fn next(&self) {
        if let Ok(conn) = Connection::session().await {
            let _ = self.call_player_method(&conn, "Next").await;
        }
    }

    pub async fn previous(&self) {
        if let Ok(conn) = Connection::session().await {
            let _ = self.call_player_method(&conn, "Previous").await;
        }
    }

    pub async fn select_source(&self, player_id: &str) {
        info!(
            "MediaService: User manually selected player source: {}",
            player_id
        );
        self.set_active_player(Some(player_id.to_string()));

        if let Ok(conn) = Connection::session().await {
            let update = self.query_current_track_and_sources(&conn).await;
            self.publish_update(update);
        }
    }

    fn active_player(&self) -> Option<String> {
        self.state
            .read()
            .ok()
            .and_then(|state| state.active_player.clone())
    }

    fn set_active_player(&self, player: Option<String>) {
        if let Ok(mut state) = self.state.write() {
            state.active_player = player;
        }
    }

    fn publish_update(&self, update: MediaUpdate) {
        if let Ok(state) = self.state.read()
            && let Some(sender) = state.events.as_ref()
        {
            let _ = sender.send(DomainEvent::Media(update));
        }
    }

    async fn call_player_method(&self, conn: &Connection, method: &str) -> Result<(), zbus::Error> {
        let players = Self::find_players(conn).await?;
        if players.is_empty() {
            return Ok(());
        }

        let current_active = self.active_player();
        let target_player = if let Some(ref act) = current_active {
            if players.contains(act) {
                act.clone()
            } else {
                players[0].clone()
            }
        } else {
            players[0].clone()
        };

        info!(
            "MPRIS: Calling method '{}' on target player '{}'",
            method, target_player
        );

        let proxy = zbus::Proxy::new(
            conn,
            target_player.as_str(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .await?;

        let _: () = proxy.call(method, &()).await?;

        // Re-query immediately to reflect UI state
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let update = self.query_current_track_and_sources(conn).await;
        self.publish_update(update);

        Ok(())
    }

    async fn resolve_identity(conn: &Connection, bus_name: &str) -> String {
        if let Ok(proxy) = zbus::Proxy::new(
            conn,
            bus_name,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2",
        )
        .await
        {
            if let Ok(identity) = proxy.get_property::<String>("Identity").await {
                if !identity.trim().is_empty() {
                    return identity;
                }
            }
        }

        // Fallback: strip prefix and capitalize
        let stripped = bus_name.trim_start_matches("org.mpris.MediaPlayer2.");
        let mut chars = stripped.chars();
        match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }

    async fn find_players(conn: &Connection) -> Result<Vec<String>, zbus::Error> {
        let dbus_proxy = zbus::fdo::DBusProxy::new(conn).await?;
        let names = dbus_proxy.list_names().await?;
        let mut players = Vec::new();
        for name in names {
            if name.starts_with("org.mpris.MediaPlayer2.") {
                players.push(name.to_string());
            }
        }
        Ok(players)
    }

    fn extract_str(val: &Value) -> Option<String> {
        match val {
            Value::Str(s) => Some(s.to_string()),
            Value::Value(v) => Self::extract_str(v),
            _ => None,
        }
    }

    fn extract_artist(val: &Value) -> Option<String> {
        match val {
            Value::Array(arr) => arr.first().and_then(Self::extract_str),
            Value::Str(s) => Some(s.to_string()),
            Value::Value(v) => Self::extract_artist(v),
            _ => None,
        }
    }

    fn url_decode(s: &str) -> String {
        let mut result = Vec::new();
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(hex_val) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    result.push(hex_val);
                    i += 3;
                    continue;
                }
            }
            result.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&result).to_string()
    }

    async fn resolve_art_path(raw_url: &str) -> String {
        if raw_url.is_empty() {
            return String::new();
        }

        if let Some(stripped) = raw_url.strip_prefix("file://") {
            let decoded = Self::url_decode(stripped);
            if std::path::Path::new(&decoded).exists() {
                return decoded;
            }
        } else if raw_url.starts_with('/') {
            if std::path::Path::new(raw_url).exists() {
                return raw_url.to_string();
            }
        } else if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
            let cache_dir = std::env::temp_dir().join("notchde-cache").join("albumart");
            let _ = std::fs::create_dir_all(&cache_dir);

            use std::hash::{DefaultHasher, Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            raw_url.hash(&mut hasher);
            let hash = hasher.finish();
            let cache_file = cache_dir.join(format!("{:016x}.jpg", hash));

            if cache_file.exists()
                && std::fs::metadata(&cache_file)
                    .map(|m| m.len() > 0)
                    .unwrap_or(false)
            {
                return cache_file.to_string_lossy().to_string();
            }

            // Download using curl asynchronously with a quick timeout
            let res = tokio::process::Command::new("curl")
                .args([
                    "-s",
                    "-L",
                    "--max-time",
                    "4",
                    "-o",
                    cache_file.to_str().unwrap_or_default(),
                    raw_url,
                ])
                .status()
                .await;

            if let Ok(st) = res {
                if st.success()
                    && cache_file.exists()
                    && std::fs::metadata(&cache_file)
                        .map(|m| m.len() > 0)
                        .unwrap_or(false)
                {
                    debug!("Downloaded album art to {:?}", cache_file);
                    return cache_file.to_string_lossy().to_string();
                }
            }
        }

        String::new()
    }

    async fn query_current_track_and_sources(&self, conn: &Connection) -> MediaUpdate {
        let players = Self::find_players(conn).await.unwrap_or_default();
        if players.is_empty() {
            return MediaUpdate::default();
        }

        let mut candidates = Vec::new();
        let mut meta_map = HashMap::new();

        for player in &players {
            let identity = Self::resolve_identity(conn, player).await;
            if let Ok(proxy) = zbus::Proxy::new(
                conn,
                player.as_str(),
                "/org/mpris/MediaPlayer2",
                "org.mpris.MediaPlayer2.Player",
            )
            .await
            {
                let status: String = proxy
                    .get_property("PlaybackStatus")
                    .await
                    .unwrap_or_else(|_| "Stopped".to_string());

                let metadata: HashMap<String, Value> =
                    proxy.get_property("Metadata").await.unwrap_or_default();

                let title = metadata
                    .get("xesam:title")
                    .and_then(Self::extract_str)
                    .unwrap_or_default();

                let artist = metadata
                    .get("xesam:artist")
                    .and_then(Self::extract_artist)
                    .unwrap_or_default();

                let has_metadata =
                    !metadata.is_empty() && (!title.is_empty() || !artist.is_empty());

                candidates.push(PlayerCandidate {
                    bus_name: player.clone(),
                    identity,
                    status,
                    has_metadata,
                    title,
                    artist,
                });

                meta_map.insert(player.clone(), metadata);
            }
        }

        let current_active = self.active_player();
        let selected = select_target_player(&candidates, current_active.as_deref());

        let active_bus_name = selected.map(|c| c.bus_name.clone());
        self.set_active_player(active_bus_name.clone());

        // Build list of MediaSource
        let sources: Vec<MediaSource> = candidates
            .iter()
            .map(|c| MediaSource {
                id: c.bus_name.clone(),
                name: c.identity.clone(),
                is_active: active_bus_name.as_deref() == Some(&c.bus_name),
                is_playing: c.status.eq_ignore_ascii_case("playing"),
            })
            .collect();

        // Build MediaTrack for the active player
        let track = if let Some(sel) = selected {
            let metadata = meta_map.remove(&sel.bus_name).unwrap_or_default();
            let title = sel.title.clone();
            let artist = sel.artist.clone();
            let album = metadata
                .get("xesam:album")
                .and_then(Self::extract_str)
                .unwrap_or_default();

            let raw_art_url = metadata
                .get("mpris:artUrl")
                .or_else(|| metadata.get("xesam:artUrl"))
                .and_then(Self::extract_str)
                .unwrap_or_default();

            let art_url = Self::resolve_art_path(&raw_art_url).await;
            let is_playing = sel.status.eq_ignore_ascii_case("playing");

            MediaTrack {
                title,
                artist,
                album,
                art_url,
                is_playing,
                position_ms: 0,
                duration_ms: 0,
            }
        } else {
            MediaTrack::default()
        };

        MediaUpdate {
            track,
            sources,
            active_player: active_bus_name,
        }
    }

    fn set_event_sender(&self, sender: UnboundedSender<DomainEvent>) {
        if let Ok(mut state) = self.state.write() {
            state.events = Some(sender);
        }
    }
}

impl Service for MediaService {
    fn name(&self) -> &'static str {
        "media"
    }

    fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle, ServiceError> {
        self.set_event_sender(ctx.event_sender());
        let shutdown = ctx.shutdown_signal().clone();
        let task_shutdown = shutdown.clone();
        let service = self.clone();
        let join = ctx.spawn(async move {
            info!("MPRIS Media service starting with multi-source D-Bus event listening");
            let conn = match Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to connect to D-Bus session bus: {}", e);
                    return;
                }
            };

            let mut last_update = MediaUpdate::default();
            let initial = service.query_current_track_and_sources(&conn).await;
            if initial != last_update {
                last_update = initial.clone();
                service.publish_update(initial);
            }

            let prop_rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .interface("org.freedesktop.DBus.Properties")
                .and_then(|b| b.member("PropertiesChanged"))
                .map(|b| b.build());

            let name_rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .interface("org.freedesktop.DBus")
                .and_then(|b| b.member("NameOwnerChanged"))
                .map(|b| b.build());

            let mut prop_stream = match prop_rule {
                Ok(rule) => MessageStream::for_match_rule(rule, &conn, None).await.ok(),
                Err(e) => {
                    error!("Failed to create D-Bus PropertiesChanged match rule: {}", e);
                    None
                }
            };

            let mut name_stream = match name_rule {
                Ok(rule) => MessageStream::for_match_rule(rule, &conn, None).await.ok(),
                Err(e) => {
                    error!("Failed to create D-Bus NameOwnerChanged match rule: {}", e);
                    None
                }
            };

            info!("MPRIS Media service subscribed to D-Bus signals (zero polling)");

            loop {
                tokio::select! {
                    _ = task_shutdown.cancelled() => break,
                    _ = async {
                        match &mut prop_stream {
                            Some(stream) => stream.next().await,
                            None => std::future::pending().await,
                        }
                    } => {}
                    _ = async {
                        match &mut name_stream {
                            Some(stream) => stream.next().await,
                            None => std::future::pending().await,
                        }
                    } => {}
                }

                let update = service.query_current_track_and_sources(&conn).await;
                if update != last_update {
                    debug!(
                        "Media update via D-Bus: {} - {} (sources: {})",
                        update.track.artist,
                        update.track.title,
                        update.sources.len()
                    );
                    last_update = update.clone();
                    service.publish_update(update);
                }
            }
        });
        Ok(ServiceHandle::new(self.name(), shutdown, join))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_do_not_share_active_player() {
        let first = MediaService::new();
        let second = MediaService::new();
        first.set_active_player(Some("org.mpris.MediaPlayer2.first".to_string()));

        assert_eq!(
            first.active_player().as_deref(),
            Some("org.mpris.MediaPlayer2.first")
        );
        assert_eq!(second.active_player(), None);
    }

    #[test]
    fn test_select_target_player_playing_takes_precedence() {
        let candidates = vec![
            PlayerCandidate {
                bus_name: "org.mpris.MediaPlayer2.zen".to_string(),
                identity: "Zen Browser".to_string(),
                status: "Paused".to_string(),
                has_metadata: true,
                title: "Threads".to_string(),
                artist: "".to_string(),
            },
            PlayerCandidate {
                bus_name: "org.mpris.MediaPlayer2.spotify".to_string(),
                identity: "Spotify".to_string(),
                status: "Playing".to_string(),
                has_metadata: true,
                title: "Song".to_string(),
                artist: "Artist".to_string(),
            },
        ];

        let selected = select_target_player(&candidates, None);
        assert_eq!(selected.unwrap().bus_name, "org.mpris.MediaPlayer2.spotify");
    }

    #[test]
    fn test_select_target_player_preserves_active_when_paused() {
        let candidates = vec![
            PlayerCandidate {
                bus_name: "org.mpris.MediaPlayer2.zen".to_string(),
                identity: "Zen Browser".to_string(),
                status: "Paused".to_string(),
                has_metadata: true,
                title: "Threads".to_string(),
                artist: "".to_string(),
            },
            PlayerCandidate {
                bus_name: "org.mpris.MediaPlayer2.spotify".to_string(),
                identity: "Spotify".to_string(),
                status: "Paused".to_string(),
                has_metadata: true,
                title: "Song".to_string(),
                artist: "Artist".to_string(),
            },
        ];

        let selected = select_target_player(&candidates, Some("org.mpris.MediaPlayer2.spotify"));
        assert_eq!(selected.unwrap().bus_name, "org.mpris.MediaPlayer2.spotify");
    }

    #[test]
    fn test_select_target_player_sticks_with_active_if_both_playing() {
        let candidates = vec![
            PlayerCandidate {
                bus_name: "org.mpris.MediaPlayer2.zen".to_string(),
                identity: "Zen Browser".to_string(),
                status: "Playing".to_string(),
                has_metadata: true,
                title: "Video".to_string(),
                artist: "YouTube".to_string(),
            },
            PlayerCandidate {
                bus_name: "org.mpris.MediaPlayer2.spotify".to_string(),
                identity: "Spotify".to_string(),
                status: "Playing".to_string(),
                has_metadata: true,
                title: "Song".to_string(),
                artist: "Artist".to_string(),
            },
        ];

        let selected = select_target_player(&candidates, Some("org.mpris.MediaPlayer2.spotify"));
        assert_eq!(selected.unwrap().bus_name, "org.mpris.MediaPlayer2.spotify");
    }
}
